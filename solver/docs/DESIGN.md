# Design: the frame solver for optimal Taxman

The frame solver ("the solver" below) finds optimal games of Taxman: for each n, a game on the numbers 1 to n that
scores as much as any game can.  With each game it records how the game's optimality is proven; section 2 says what
"optimal" means here.

This page explains what the solver does, and why.  It reads from top to bottom, in the order of the contents below, and
defines each term where it first matters.

The other documents:

* [STATUS.md](STATUS.md) is the current state of the work: what is in flight, paused, decided and waiting.  Read it
  first.
* The [README](../frame/README.md) is the user guide: how to build and run the solver.  The [reference](REFERENCE.md)
  has every flag and command, the output, and what a result records.  The [glossary](../../docs/GLOSSARY.md) defines the
  project's words.
* The [lab notebook](../notebook/README.md) says what each part was worth and what was tried instead.  Its front page is
  the register of everything tried, kept or not; each entry is a folder in [solver/notebook/](../notebook/).
* In the code, each module starts with a comment that says what it is for, and [`src/lib.rs`](../frame/src/lib.rs) has
  the module map.

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

## 1. Goals and constraints

* **Go past n = 1000**, where the Java solver's table of optimal games stopped, as far as the machines at hand reach: a
  laptop (an Apple M1 Max: 10 CPUs, 32 GB of RAM, a fast SSD) and a cloud machine of many CPUs ([RUNBOOK](RUNBOOK.md)).
  [results/README.md](../../results/README.md#strictjsonl) says how far the committed chain of games reaches, and which
  machine made each part of it.
* **Be right before being fast.**  A game counts only once an independent
  [referee](../../docs/GLOSSARY.md#results-and-trust) has replayed it.  Every result records how its optimality is
  proven and what that proof takes as given.  Every [tactic](../../docs/GLOSSARY.md#the-search) is proven or sound
  (section 5), and can be switched off to measure its effect on speed.
* **Search an enormous tree efficiently.**  Each node is fast on a single thread.  The search scales cleanly to the CPUs
  available, up to a 192-CPU machine, without depending on many of them.
* **Never rely on everything fitting in RAM.**  The configuration bounds memory use, whatever the size of the tree.
  Work that does not fit goes to disk.
* **Lose little to a restart.**  A machine can restart without warning, and a job that writes its output only at the end
  then loses all of its progress.  So every long run checkpoints often and resumes after a restart, losing at most one
  checkpoint interval.  Stopped by SIGINT or SIGTERM, it loses nothing.
* **Iterate fast.**  Correctness and performance are worked out on small and medium games that finish in seconds: the
  tests, the benchmark set and the probe (section 10).  Big runs come last.
* **Standard library only**: no third-party crates.  The solver runs on Linux and macOS.

## 2. The game and the frame model

The solver does not search the game move by move.  It splits the numbers into independent groups, the
[frames](../../docs/GLOSSARY.md#the-frame-model), settles what it can at setup, and then searches only over which spare
numbers to [promote](../../docs/GLOSSARY.md#the-frame-model).  The [glossary](../../docs/GLOSSARY.md) defines each term
and the [README](../frame/README.md#how-the-solver-sees-the-game-the-frame-model) walks through the board 1 to 21; this
section names the code that does each step, and says what the solver's answer means.

### The game

The rules are those of the glossary's [game](../../docs/GLOSSARY.md#the-game).  In the code, `game::replay` applies them
to a list of moves.  Its companion `game::replay_finished` also requires the game to be over and to open with the
largest prime, as every optimal game does.  Together they are the [referee](../../docs/GLOSSARY.md#results-and-trust) of
a game, and they share no code with the solver.

### Sources, factors, frames

On a board, the [sources](../../docs/GLOSSARY.md#the-frame-model) are the numbers above n/2 and the
[factors](../../docs/GLOSSARY.md#the-frame-model) the rest.  A move is paid for by one of its [maximal
factors](../../docs/GLOSSARY.md#the-game), and a factor pays for one move.  Although a move takes every divisor of it,
maximal factors are enough: see the lifting argument of `numtheory` and [THEORY.md](../../docs/THEORY.md) (Theorem 1),
which the [README](../frame/README.md#how-the-solver-sees-the-game-the-frame-model) summarises.

`frames::BoardFrames` forms the [frames](../../docs/GLOSSARY.md#the-frame-model) of a board, in playing order: the
frames [in front of](../../docs/GLOSSARY.md#the-frame-model) a frame are played before it.  Setup simplifies each frame
in three ways.  It finds the frame's [mandatory moves](../../docs/GLOSSARY.md#the-frame-model) (`unzip`), drops its
[impossible sources](../../docs/GLOSSARY.md#the-frame-model) (`removeImpossible`), and counts its
[freedoms](../../docs/GLOSSARY.md#the-frame-model); a [closed](../../docs/GLOSSARY.md#the-frame-model) frame has none.

`frames::build_frames` numbers the frames that have freedoms by [layer](../../docs/GLOSSARY.md#the-frame-model), from
the front, and merges the frames of one [frame number](../../docs/GLOSSARY.md#the-frame-model), so that a promotion
always has one frame to go to; the README's example shows where that happens.  The glossary's [frame
number](../../docs/GLOSSARY.md#the-frame-model) compares the numbering with the Java's.

Each frame plays [Taxman Mini](../../docs/GLOSSARY.md#the-frame-model), and is
[winnable](../../docs/GLOSSARY.md#the-frame-model) when some order takes every move.  The module `model` tests that with
counters instead of searching orders, by [peel](../../docs/GLOSSARY.md#the-game).

### Promotions and the score

Once setup has settled the frames, what a game can still choose is which spare factors to
[promote](../../docs/GLOSSARY.md#the-frame-model).  The score of a game of the model is `in_the_bag`, what the game with
no promotions scores, plus the sum of its promotions.  So the solver looks for the [promotion
set](../../docs/GLOSSARY.md#the-frame-model) with the largest sum.  The moves of the game follow from the promotions
(`frames::BoardFrames`, with the promotions treated as sources).  The numbers that could be promoted are the
[candidates](../../docs/GLOSSARY.md#the-frame-model), and the glossary has the rules of a promotion: one frame forward,
a freedom spent, the frames reduced.

### What "optimal" means

The solver's answer for n rests on up to three premises: the frame model is exact at n, the search's pruning is exact,
and the game of n − 1 is optimal.

**Within the frame model.**  The search finds [P\*](../../docs/GLOSSARY.md#the-frame-model)(Π_n), the largest sum of a
promotion set of the root problem Π_n (section 8, "Same search", defines the [search
problem](../../docs/GLOSSARY.md#the-frame-model) Π exactly).  The answer is in_the_bag + P\*(Π_n).  [THEORY.md, "The
frame model: what is proven and what remains"](../../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains)
proves what that is:

* The sources that setup keeps are [U\*](../../docs/GLOSSARY.md#the-frame-model), chosen greedily from the largest on
  each frame, and in_the_bag = ΣU\* (Theorem M, M1).
* **Completeness and soundness** (Theorem M): a set U\* ∪ L, with L a set of numbers up to n/2, is playable exactly when
  L is a promotion set of Π_n, and then it scores in_the_bag + ΣL.  So no step of setup loses a game that plays U\*, and
  none admits a set that is not a game.  The steps are `unzip`'s mandatory moves, freedoms = factors − sources, closed
  frames taking no part, a promotion paying one frame forward, the numbering, the candidate filter and the root's
  reduction.  What the proof needs of the numbering is that it is [clean](../../docs/GLOSSARY.md#the-frame-model), which
  the numbering by layer is by construction.
* Every optimal game plays the members of U\* with one or two prime factors (Theorem B).
* So in_the_bag + P\*(Π_n) = opt(n) exactly when [statement U](../../docs/GLOSSARY.md#the-frame-model) holds at n: some
  optimal game plays the members of U\* with three or more prime factors.

The statement U is open in general.  It holds at every game that the
[prover](../../docs/GLOSSARY.md#programs-files-and-documents) has proven, since the prover proves opt(n) on the whole
board; [results/README.md](../../results/README.md#what-supports-the-scores) says how far its proofs of the committed
chain reach.  The prover ([its README](../noframe/README.md)) is a separate program that uses nothing of frames; section
3 of its README lists the facts that its proofs use.

Past the prover's proofs, the committed games are proven within the frame model, so each is opt(n) wherever U holds at
n.  Proving U at such an n takes two steps, which the command `taxman certify` combines game by game ("Certificates of
U", section 8); [results/README.md](../../results/README.md#certificates) says which games have certificates.

First, the prover's [barred bounds](../../docs/GLOSSARY.md#bounds) settle most members of U\*: each member that a barred
bound certifies is played by every set of moves that beats the recorded score and obeys Theorem B, and the members it
does not certify are *open*.  It certifies most members in each game, within the prover's proofs and past them, but
closes no game alone ([the prover's README](../noframe/README.md#the-upper-part-game-by-game) has the method, and [the
entry](../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) the shares).

Second, a search settles the open members.  It is the search of the [extended
problem](../../docs/GLOSSARY.md#the-frame-model) Π⁺(O), with the set O of open members optional (THEORY.md, Theorem M⁺;
`frames::build_extended`, research code).  The extended problem is exact like Π_n, so a search of it that finds nothing
better, with the other members certified, proves opt(n) on the whole board.  The search takes the open members as
one-sided candidates (section 4, "Extended mode"; `Board::prepare_extended_with`).

Details of the extended problem: it keeps the claims of `unzip` that Lemma M5⁺ (THEORY.md) allows.  With nothing open
that is all of them, so Π⁺(∅) is Π_n itself.  Without the claims, Π⁺(∅) is the frame model with them put back
(`examples/extended.rs --no-claims`).  A hunt for a counterexample on the boards without each open member, the example
`hunt`, was prepared and never run.  It is removed from master, and [its
entry](../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md) says where it builds ("the
program removed from master").

What each kind of proof needs of the frame model, beyond the code's correctness and the games before it:

| proof | needs |
|---|---|
| `prime_formula` | nothing of the model (proved at `accelerate::prime_shortcut`) |
| `cap` | nothing of the model: the game is replayed, and reaches the ceiling n + score(n − 1) ≥ opt(n) given that score(n − 1) = opt(n − 1) |
| `exhausted`, `trivial` | completeness at n, so U at n |
| `same_search` of an exhausted game m | U at n, and Π_n = Π_m (checked) |
| `same_search` of a `cap` game m | U at n, Π_n = Π_m (checked), and soundness at m (Theorem M), which makes m's ceiling P\*(Π_m) |

The tests tie the theorems to the code:

* `tests/model_exact.rs` checks Theorem M both ways, on every set of every board to 100 and on random sets to 8191,
  against a playability oracle that shares no code with `frames` or `model`.
* `tests/model_extended.rs` checks Theorem M⁺ the same way, with sources open.
* `tests/bottom_layers.rs` checks Theorem B on every recorded and enumerated optimal set.
* Two exhaustive searches agree with the frame model's games on small boards: `brute`, the solver's own, to 40 in the
  test suite, and `heuristics/evaluation/certify.py`, to 62.  The example `sacrifice`, removed from master, once ran
  `brute` to 52 ([the entry](../notebook/the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md) has the
  run).

The sacrifice bound (`sacrifice`) is an upper bound on opt(n) by a different route: games on punctured boards, boards
with holes, where Theorem M holds too.  But U and Theorem B are statements about the board 1..n, so the bound rests on
the upper part of each punctured board.  It proves game 1133 and disagrees with nothing.

**Given the pruning is exact.**  Every [tactic](../../docs/GLOSSARY.md#the-search) is proven or sound; section 5 grades
each.  The Java used two shortcuts that rest on conjectures, `reuse_prev` and `revise_freedoms`.  The frame solver had
them as tactics for a while, measured that they no longer paid, and removed them ([which tactics earn their
keep](../notebook/which-tactics-earn-their-keep-2026-10/README.md)).

**Given the previous game is optimal.**  The game of n is built from the game of n − 1, using the
[ceiling](../../docs/GLOSSARY.md#the-game) and the [prime formula](../../docs/GLOSSARY.md#the-game).  So a chain of
games from n = 1 is proven game by game (section 8).

Optimal games need not be unique, though, and that matters to any step that reuses the game of n − 1.  From n = 425 on,
some games have two or three [optimal sets](../../docs/GLOSSARY.md#the-game).  In all of them the members above n/2 are
U\*; they differ below n/2.  For example, the recorded game of 685 plus the move 686 is unplayable, although opt(686) =
686 + opt(685): the equality holds through another optimal set of 685.  So a chain step is decided by the complete list
of the optimal sets of n − 1, not by one of them ([the prover's README, section
5](../noframe/README.md#5-is-the-optimal-set-unique)).

### The Java solver and the frame solver

The frame solver uses the Java solver's strategy, which the [wiki](https://github.com/bvchess/taxman/wiki) describes:
split the game into frames of Taxman Mini, and search for the best promotion set.  The Java is research code, three
generations of the same frame idea layered on each other.  So the frame solver reimplements the *algorithm*, designed
for clarity and speed, instead of translating the classes.

What must match the Java is the *results*.  The setup (frames, candidates, the greedy incumbent) defines the search
space, and it reproduces the Java's values exactly; the Java's golden data in `testdata/golden/` is the check.  Every
final score equals optimal.json's.

The search is the frame solver's own.  It started as the Java's loop: its `blameGame` bound, its depth-2 preclusion
table, the promotability checks of its candidate lists, and branching on the largest candidate.  Each of those was
replaced by something measured to be faster, then removed.  The notebook has the rounds, and [which tactics earn their
keep](../notebook/which-tactics-earn-their-keep-2026-10/README.md) has the last measurements of what was removed.  Not
ported: `oldsearch/` (the 2021 disk-queue search), `frame.GreedySolver`, `util.Memo`, `WorkingFrame.fancyFit/fits`, the
picklers and the compressor.

The Java's and the wiki's words map onto this document's:

| the Java or the wiki | here |
|---|---|
| `Hive` | a frame during setup, before or after the frames of one number are merged (`frames::Frame`) |
| `Apiary` | all the frames of a board in playing order (`frames::BoardFrames`) |
| `FrameBuilder`'s levels | the frame numbers |
| `WorkingFrame`, `search.Frame` | a frame as the search takes it (`model::Frame`) |
| `freeFactorCount` | the freedoms |
| `downstream`, `upstream` | in front of, behind |
| `fits` | promotable |
| `getMaxPromotionSum` | the ceiling as a promotion sum |
| `isWinnable`, `inTheBag`, `maxTotalPromotions`, `promotionCandidates`, `mandatoryMoves` | used as they are |
| the wiki's *selection* | a move |
| the wiki's *front* and *back* | the same |
| the wiki's upper bound N + opt(N − 1) | the ceiling |

The port found one defect in the Java's bound: it can be too low.  Its `UpperBoundEstimator.getRunnerUp` replaces the
failing number with the next candidate below the smallest bid member, but a skipped candidate from the failing number's
frame can be larger.  At the root of n = 114, the Java's bound is 358 against the best completion's 360 ([the
story](../notebook/the-port-and-its-first-improvements/README.md)).

## 3. Architecture

The frame solver is one Cargo crate, `solver/frame/`: a library, the binary `taxman`, examples and integration tests,
with no third-party dependencies.  The library's modules form layers, each using only the layers before it, and a solve
passes through all of them but the studies.  The module map of [`src/lib.rs`](../frame/src/lib.rs) lists the modules in
that order, a line each.  The list below follows the same order, with advice for a first read.  Each module starts with
a comment that says what it is for; read it first.  The pipeline of section 7 shows how the pieces fit together, so keep
it open.

**Foundations.**

1. `src/game.rs`: the rules as a referee, and `optimal.json`, the table of optimal games.
2. `src/numtheory.rs`: maximal factors and composites; and `src/playable.rs`, whether a set of numbers can all be played
   on the whole board, with nothing of the frame model (the oracle of the model's tests, and the referee of
   `taxman certify`).
3. `src/bits.rs`: `Bits<W>`, the fixed-width bitsets of game numbers that everything above is generic over, and the set
   widths.  The module comment is enough at first.
4. `src/json.rs`: the small JSON reader and writer.  Skim it; it matters once you read results lines.

**The frame model.**

5. `src/frames/`: forming and simplifying the frames, numbering them and merging them.  The module comment of `mod.rs`
   tells the story (sources, factors, `unzip`, impossible moves, freedoms, numbering, merging) and builds
   `SearchFrames`; `board_frames.rs` has `Frame` and `BoardFrames`.
6. `src/model/`: the search state.  `mod.rs` says what is where: `Frame` and `FrameSet` (`frame.rs`), `FrameGraph`,
   winnability and reduction (`peel.rs`), promotion (`promote.rs`), then conflicts (`conflict.rs`) and nogoods
   (`nogoods.rs`).
7. `src/accelerate.rs`: the shortcuts from the game of n − 1 (the comment of `prime_shortcut` proves the prime formula),
   and the last step, from promotions to moves.

**The search.**

8. `src/matchbound.rs`: the matching bound; its module comment is the best explanation of it.  Its child
   `src/matchbound/lagrange.rs` is the Lagrangian bound over the nogoods, built on the same levels and repairs.
9. `src/gf2.rs`: the GF(2) bound, the default node bound.  The module comment explains the two frame matroids, how the
   search keeps a node's matroids by contraction along its path, and the forced members; `src/gf2/meet.rs` is their
   weighted intersection (the tableau, the augmenting paths, the weight split).  `docs/THEORY.md`, Theorem 11, is the
   proof.
10. `src/search/`: the searcher.  `mod.rs` explains the step and says what is where: `searcher.rs` (`TaxSearcher` and
    `Stepper::step`, whose phases are `check_node`, `choose_branch`, `make_child` and `check_child`), `refine.rs`
    (`Refiner`: conflicts, refinement; `strongest`), `lagrange.rs` (the Lagrangian bound in the search: where it
    applies, its forced members, its refits), `symmetry.rs` (the rule of symmetric pairs), `dual.rs` (the dual bound of
    a stopped search), `setup.rs` (`prepare`), `tactics.rs`, `incumbent.rs`, and `header.rs` (the checkpoint header).
    The `if let Some(p) = &mut self.probe` lines in `searcher.rs` are instrumentation (`diagnostics.rs`), and a first
    read can skip them.
11. `src/bounds.rs`: two small functions, the limit on a node's further promotions (`max_additional`) and the greedy
    incumbent.

**The engine.**

12. `src/engine.rs`: the problem-independent parallel depth-first driver.  Its module comment is the protocol
    (conservation, splitting, checkpoint, termination).

**Solving.**

13. `src/tactics.rs`, `src/board.rs`, `src/solver.rs`, `src/results.rs`, `src/store.rs`, `src/known.rs`,
    `src/certify.rs`, `src/paths.rs`: the switchable tactics, a board prepared for a search, the per-game pipeline,
    provenance and verification, the bounds store's line and the relations between games, the known games that
    `--use-prev` draws on, the certificates of `taxman certify`, and the default files.  The range driver is
    `src/solver/window.rs`, the window of games in flight, and its child `window/allot.rs` shares the threads among
    them.

**The front end.**

14. `src/main.rs`, `src/cli.rs`, `src/commands.rs`: the command line, with the commands `solve`, `regress`, `bench`,
    `verify-optimal`, `verify-results`, `extend-optimal`, `tables`, `certify` and `verify-certificates`.

**Studies**, which the solver does not use and a first read skips:

15. `src/brute.rs` (exhaustive search, the test oracle) and `src/sacrifice.rs` (the sacrifice bound and punctured
    games).

**Beside the library.**  `examples/` holds the programs that use the library but are not the solver:

* built by `cargo build --examples`: the ladder `ladder` (standalone searches of hard games with time limits, thread
  scaling and diagnostics), the probe `perf` (the hot paths, replayed from fixtures and measured in cycles and
  instructions), and `ledger` (bounds on opt(n) for many games at once, tightened by limited searches and the relations
  between games; section 8, "The ledger");
* behind the Cargo feature `studies` (`--features studies`), the study tools: `tune` (the tuning benchmark: units of a
  search at the end of the chain, each solved on one thread), `structure` (the census of the frames' structure, game by
  game), `extended` (chains and problems of the extended problem) and `wall` (what grows with n and with a tree: a
  game's fixed costs piece by piece, a node's cost at any set width, large trees with their memory and checkpoints, the
  frames of any game, and the `landscape` of a range of games, each solved standalone: the root's GF(2) bound over the
  incumbent a search starts from).

`tests/` holds the integration tests (section 9), and `solver/tools/` the scripts around the programs: the second
verifiers in Python, the quick check, the profile-guided build and the cloud scripts ([solver/README.md, "The
tools"](../README.md#the-tools)).  The programs of finished studies (`fields`, `matroid`, `dominance`, `slope`,
`repeats`, `promotions`, `diff`, `hunt` and `sacrifice`) are gone from master; [the
notebook](../notebook/README.md#how-this-notebook-is-kept) says where they build.

**Width dispatch.**  A set of game numbers is a bitset of W words of 64 bits, and everything that holds one is generic
over `const W: usize`.  The macro `with_width!` (in `bits`; the solver, the examples and the tests use it) picks the
narrowest width of `bits::WIDTHS` that holds n:

| W | 8 | 16 | 20 | 24 | 28 | 32 | 36 | 40 | 48 | 56 | 64 | 128 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| n up to | 511 | 1023 | 1279 | 1535 | 1791 | 2047 | 2303 | 2559 | 3071 | 3583 | 4095 | 8191 |

Every set operation loops over all W words, so a narrower set makes every step cheaper, and the steps of 4 to 8 words
between 1023 and 4095 pay.  Past 4095 they would not: a node at 4100 costs 1% more at 128 words than at 72, with the
board layer the same, so 128 is the only width there ([boards past
4095](../notebook/boards-past-4095-2026-10/README.md)).  The price is build time, since each width is a copy of the
search's code: a release build takes about 100 seconds on an M1 Max, about 12 more for each width ([the
engine](../notebook/the-engine/README.md) has the times as widths were added).  The largest game, 8191 (`solver::MAX_N`,
from the widest width), also sets the other fixed sizes, each checked on every game to 8191: at most nine frames
(`model::MAX_FRAMES`), a peel's worklist of two summary words, and GF(2) vectors of at most five words (section 4, "The
GF(2) bound").

*Details.*  A checkpoint records its width, and a build refuses a checkpoint made at another width than it gives the
game.  So a checkpoint of a game of 2048 to 3583 made at 64 words, by a build before the widths of 36, 40, 48 and 56
words, does not resume on a build that has them.  Adding 128 moved no game, so the checkpoints of the games to 4095
resume on the builds past it.

## 4. The search

The search is a branch and bound over promotion sets.  It keeps the best promotion set found so far, the *incumbent*,
and explores the sets that could beat it, pruning every part of the tree that a bound shows cannot.  This section builds
it up: the state and the order of the search, three bounds on a node (the matching bound, the Lagrangian bound over the
nogoods, and the GF(2) bound, the default), the conflicts and nogoods that the branching rests on, and the rule of
symmetric pairs.  "The default step" then puts them together, and the last two parts are the search's start and its
extended mode.  The [glossary](../../docs/GLOSSARY.md#the-search) defines the search's words.

### Search state and order

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
`best` is the incumbent's promotion sum.  A node's loop is the Java's (`BranchExplorer.explore`), made lazy, so that the
search explores each child as soon as it makes it, instead of generating them all first.  The loop ends when a bound on
the node's completions is below its need.  Otherwise it takes a candidate out of the loop, promotes it in a child,
narrows the child's candidates, and explores the child if the child's own bound reaches the child's need.  A bound that
only equals the incumbent cannot lead to a better one, so the loop ends then too, where the Java ended only below it.
The bounds, the choice of candidate and what the child promotes are the frame solver's own: the parts below build them
up, and "The default step" puts them together.

### The matching bound

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
the Java's bound; on the former benchmark games (n = 480 to 760), 107 to 292 against 363 to 769.  The Java's `blameGame`
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

### Conflicts of the matching bound

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
  The member is the one whose removal lowers the matching bound most.  That takes one bound without each member, and
  `bound_excluding` settles most of them from the potentials alone.  A node with no conflict among its candidates
  branches on its largest.  The smallest member, the largest, and the Java's largest candidate were once tactics; at the
  hardest games they took 6 to more than 17 times as long, and were removed.
* **Exact matchings.**  A heaviest matching that can be promoted is the node's best completion: the search offers it as
  the incumbent, and the node is done.
* **Propagation into children.**  A conflict known at a node, of which a child promotes all but one member, rules that
  member out of the child's candidates: the child's state is the node's with more promotions, so promoting the member
  would promote the whole conflict.  Without this rule the child's matching bound would equal the node's.

"The starting incumbent, the ceiling and the seed", below, says where the incumbent starts.

### Forced members and nogoods

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
  over](../notebook/carrying-nogoods-over-2026-10-measured-not-kept/README.md)).  Which nogoods a searcher keeps, and
  for how long, is a policy and nothing more, since any subset of valid nogoods is as sound as all of them.  Three
  research tactics set it (`nogood_period`, `nogood_keep` and `nogood_limit`; section 5), whose defaults are the
  behaviour above, with the same trees.  The table counts what it does (nogoods learned, lookups and the share they
  answered, candidates ruled out, the table's size at each forgetting and what was kept, evictions), and the ladder and
  the progress lines print the counts.  A checkpoint does not record the policies, since the answer does not depend on
  them and a resumed search starts with empty tables; the run that resumes says which it uses.
* **Single-member conflicts belong to the node.**  Many conflicts have one member, which looks like a candidate that
  cannot be promoted from the root at all and could be filtered out once.  It cannot: on every game from 2 to 1230 the
  frame builder's candidates all pass that test at the root, which a test checks.  A single-member conflict is a
  candidate that cannot be promoted *once the node's promotions are made*: its frame may have used it to pay for a move
  left with one live factor, say.  In another branch it is part of a nogood of two or more members.  The promotions that
  fail are under 4% of the steps, so little would be saved ([single-member
  conflicts](../notebook/single-member-conflicts-belong-to-the-node-2026-10/README.md)).

### The Lagrangian bound over the nogoods

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
  nogoods](../notebook/a-lagrangian-bound-over-the-nogoods-2026-10/README.md)).  With the GF(2) bound it changes no tree
  measured (below).

### The GF(2) bound

The *GF(2) bound*, the default node bound (tactic `gf2`), relaxes winnability less than matchability does, and so prunes
more: at the roots of 880 to 1300 its gap is a tenth of the matching bound's ([matroid intersection
bounds](../notebook/matroid-intersection-bounds-2026-10/README.md)).  In each frame, give the factors coordinates over
GF(2), each *mover* (a move of the frame, or a number promoted into it) the vector of its maximal factors in the frame,
and each number promoted out of it its own unit vector.  A winnable frame peels completely, and the peel's order makes
these vectors linearly independent.  The moves of a frame are fixed, so its condition is a linear matroid on the
candidates that touch it, contracted by the moves, whose rank is the frame's freedoms.  A candidate of home `h` touches
frames `h` and `h − 1`, one even and one odd.  So the even frames' matroids are the blocks of a matroid `A` on the
candidates, the odd frames' the blocks of a matroid `B`, and every promotable set is independent in both.  The heaviest
common independent set of at most `max_add` members (weighted matroid intersection, solved in polynomial time), the
node's *heaviest set* `I*`, bounds its completions.

[THEORY.md](../../docs/THEORY.md#the-frame-matroids-a-bound-on-the-promotions), Theorem 11, has the proof; the theorem
is the frame model's form of Theorem 10, the parity matroids' bound on the whole game, which the prover uses.  The
module comments of `src/gf2.rs` and `src/gf2/meet.rs` have the algorithms.  The GF(2) bound is never above the matching
bound, since independent vectors have a system of distinct representatives, which is a matching.  What it misses are the
*odd* cores of movers with three or more factors.

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
  the span.  It derives a node's from a copy of its own tableau, saved when it kept the certified level (`Snapshot`).  A
  block where an exchange would change the span is built instead, and the elements the parent did not have (its own
  loops, candidates again in the child) are reduced by the block's members.  The module comment of `src/gf2/meet.rs`
  ("Deriving a tableau") has the argument, and the random paths of the tests check every derived tableau against the one
  a solve builds.  The trees are the same.
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
  Bellman–Ford, each round from the vertices that changed.  Its bound is loose, so for the members it leaves free within
  the tactic `force_window` of the need (all of them, by default), the exchange graph decides.  A member is forced too
  when no cycle through it is shorter than the slack `value − need` and the sets of other sizes lose more than that
  (`Meet::has_cycle_within`).  A member on a cycle that the search for another found within the slack is not searched,
  since that cycle passes through it.  The window `all` costs each node more, since every member is checked exactly, and
  the trees fall by more: at the end of the chain it takes 0.41 to 0.43 of the nodes for 0.60 to 0.63 of the CPU of the
  former window, 80 (`gf2::LEGACY_FORCE_WINDOW`; [the tuning
  campaign](../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).  The searches reach the
  elements free in `A` from the members, and the members from the elements free in `B`, through one hub each (the least
  distance plus `w1`, less `w2`), which keeps their distances, order and cycles.  Their queue is a radix heap of packed
  (distance, vertex) keys, which come out in the binary heap's order.
* **Exact strong branching.**  The member of the conflict to branch on is the one without which the intersection is
  lowest.  The search reads each such bound off the exchange graph where it certifies it, and solves it where not: in
  the order of the split's bounds, each solve stopped once it reaches both the lowest so far and the need, after a
  greedy lower bound.  A member whose bound falls below the need is forced too, and the chosen member's level, certified
  or solved, is what the node goes on with.  The cheaper choices (the split's bound alone, the largest member) take more
  nodes and more time ([the GF(2) bound in the search](../notebook/the-gf2-bound-in-the-search-2026-10/README.md)).  The
  tactics `branching`, `strong_limit`, `strong_depth`, `deep_branching` and `more_forced` choose among them, and set how
  far strong branching goes (section 5).
* **What it replaces.**  With `gf2` the search does not compute the matching bound, which as a first check before the
  intersection costs more than the cuts it saves; so `match_refine` has no effect.  The conflicts, nogoods, forced
  members and exact sets work as with the matching bound.  The Lagrangian bound over the nogoods still applies after the
  GF(2) bound when it is on, but changes no tree of the GF(2) search measured and costs 2 to 4%, so `lagrange` is off by
  default.
* **Block words.**  A level's vectors are as wide as the root's largest frame rank (its freedoms) needs: one to three
  words of 64 bits on every game to 4095 (ranks up to 169), four from 4740 and five from 6392 (ranks up to 314 at 8191;
  [boards past 4095](../notebook/boards-past-4095-2026-10/README.md) has the scan of every game).  Each word count is a
  copy of the bound's code (`Levels<W, R>`, R from 1 to `gf2::MAX_WORDS` = 5) for every set width that has it.  Only the
  sets of 128 words have the copies of four and five words, so the narrower widths build as before.  Tests run those
  copies on small games with the vectors padded with zero words (`Gf2Graph::padded`), where they must search the same
  tree.
* **Open for other weights.**  `meet::Problem` takes any positive weights, but two of its starts assume that weights do
  not decrease with an element's index, as the root's candidates, in increasing order, have them: the start from the
  heaviest uncontended elements, and the greedy lower bound.  A bound that weighs the candidates otherwise (a Lagrangian
  over the nogoods, or a cover LP with this intersection as its inner problem, as the nogoods study suggests) needs its
  own element order, or those two starts switched off.  The levels, the contraction and the split do not depend on the
  order.

### Prime exchanges

The rule of symmetric pairs (tactic `symmetry`, on by default) prunes the parts of the tree that are mirror images of
others.  Exchanging two primes `p < q` in every number (`p^a q^b m ↦ p^b q^a m`) is an automorphism of divisibility.
Where the exchange maps every frame of the root onto itself (moves to moves, factors to factors) and the candidates onto
themselves, the pair is *symmetric*, and the exchange maps promotion sets to promotion sets worth `(q − p)(c_p − c_q)`
more.  Here `c_r(S)` is the sum of `x / r` over the members `x` of `S` divisible by `r`; in the pairs used, no candidate
is divisible by `pq`, `p²` or `q²`.  So every heaviest promotion set keeps `c_p ≤ c_q` for every symmetric pair at once:
a set that breaks one pair's inequality is beaten by its image under that pair's exchange alone
([docs/THEORY.md](../../docs/THEORY.md#prime-exchanges-symmetries-of-the-promotions), Theorem 13).

The pairs come in classes of primes that the frame builder treats alike.  At 1300 the classes are {41, 43}, {59, 61},
{67, 71}, {73, 79}, {89, …, 107}, {109, …, 139} and {149, 151, 157}, which make 32 pairs; there are 99 at 2175 and 300
at 4000.  `search::symmetry::root_pairs` finds them when a search is set up, in under a millisecond: it makes the full
check only for the primes whose multiples have the same places and exponents in the frames, and only over the
multiples.  [Dominance and symmetry](../notebook/dominance-and-symmetry-2026-10/README.md) found the exchanges, and
[prime exchanges in the search](../notebook/prime-exchanges-in-the-search-2026-10/README.md) put them in.

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

### The default step

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
facts (section 9).

### The starting incumbent, the ceiling and the seed

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
ceiling before each pass, so a ceiling given while it runs (through the control, section 6) ends it as soon as its set
reaches it.  The start is offered to the run's control as it is made (after the greedy set and the seed, after the dive,
after the local search), so whoever steers the search sees the best so far before the engine starts, and the engine's
start is unchanged.  `board::Board` sets a board and its search up: it is the one place that knows the path from the
tables to the frames, the prepared search, the ceiling and the seed.

*A floor.*  A search can start instead from a *floor* (`search::search_above`): an incumbent worth a given promotion
sum, with no set and no start made.  The search prunes nodes against the floor as against a set of that weight, and a
set it finds heavier becomes the incumbent as usual.  A search that completes with none found has proven that nothing
is heavier than the floor ([docs/THEORY.md](../../docs/THEORY.md), Lemma F, the argument of the dual bound of a stopped
search), which `SearchResult::floor_held` says.  Its tree is that of a search from an incumbent of that worth: at P\*
the proof's tree, node for node, as a search seeded with an optimal set makes it; above P\* a smaller one, by a factor
of about 27 for each quarter of the gap between the root's bound and P\*, on the games measured ([searches under a
floor](../notebook/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md)).

### Extended mode: one-sided candidates

The search can also take the *extended problem* Π⁺(O), the problem that a certificate of U searches (section 8,
"Certificates of U").  It is the frame model with a set O of the sources of three or more prime factors that setup
keeps made optional (THEORY.md, ["The extended problem"](../../docs/THEORY.md#the-extended-problem), and the
corollary of Theorem M⁺; `frames::build_extended`).  Besides the candidates of the frame model, which are *two-sided*
(out of one frame, into the one in front), it has *one-sided* candidates: the members of O and the sources that greedy
rejected beside them, each a possible move of its *own* frame.  It is research code: the solver's own path never
builds an extended problem, and with no one-sided candidate every piece below is what it is without them.

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
  another.  A checkpoint of version 4 resumes only a search without them (section 8).
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
  problem](../notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)).  Opening a source also adds one
  to its frame's rank, so a problem with many open sources can need more words of the GF(2) bound than the narrow set
  widths compile (three, rank 192) or the widest (five, rank 320): `search::prepare_extended` refuses it, saying so,
  rather than fail in the search.

## 5. Tactics

A *tactic* is a switchable piece of the search: each row below is one, in `tactics::Tactics`, set from the command line
with `--tactic name=value`.  Every tactic is *proven* (a theorem, given a premise) or *sound* (it never prunes a
promotable optimal set), so switching one off changes how fast an answer comes, never the answer.  The values are `on`
or `off`, but for `match_refine` (`0` to `7`), `nogood_period` and `nogood_limit` (whole numbers), `nogood_keep` (a
policy), and the tactics that tune the GF(2) search, `force_window` to `more_forced`, which their rows give.  The
tactics that act inside the search are the fields of `search::SearchTactics`, and `examples/ladder.rs` takes the same
names for them.  A checkpoint records the search tactics in its header (section 6), but for the three that set how the
nogoods are kept.  A test (`tests/docs.rs`) checks that this table lists the tactics of `tactics::Tactics`, in order,
with their defaults.

| tactic | default | status | effect |
|---|---|---|---|
| `prime_shortcut` | on | proven (given an optimal game of n − 1: the proof is at `accelerate::prime_shortcut`) | a prime n plays the game of n − 1 with its first move replaced by n |
| `same_search` | on | proven (given the proof of the earlier game: section 8, "Same search") | a game whose search problem an earlier game of the chain posed and settled is that game's promotions on this board, with no search (method `repeat`, proof `same_search`) |
| `match_refine` | 1 | sound (see `search::refine::Refiner`) | where the matching bound does not prune, refine it by branching on conflicts this many levels deep (0 = off, at most 7); no effect with `gf2`, which does not use the matching bound |
| `fix_forced` | on | sound (every completion that beats the incumbent promotes them) | a node's child promotes all its forced members, and the node's loop ends after that child |
| `nogoods` | on | sound (a nogood holds at every node; see `model::Nogoods`) | learn each conflict as a nogood of the root, and use the table at every node: for the conflict to branch on and refine with, and to rule candidates out of children |
| `forget_nogoods` | on | sound (the table is a cache) | with `nogoods`, each searcher forgets every nogood it knows after each `nogood_period` of its steps, or keeps some (`nogood_keep`); off: never |
| `nogood_period` | 50000 | sound (the table is a cache) | with `nogoods` and `forget_nogoods`, the steps of a searcher between its forgettings (a whole number from 1); a research knob, with the next two (section 4, "Forgetting nogoods") |
| `nogood_keep` | none | sound (the table is a cache) | what a forgetting keeps: `none` (nothing), `hit` (the nogoods used since the last forgetting), `sizeN` (those of at most N members), `topN` (the N most used since the last forgetting, of those used at all) |
| `nogood_limit` | 0 | sound (the table is a cache) | with `nogoods`, a searcher's table holds at most this many nogoods, the least recently used dropped to make room for a new one (0: no limit) |
| `lagrange` | off | proven (weak duality, for any multipliers over valid nogoods; `matchbound::lagrange`) | with `nogoods`, where the node bound leaves a node open, bound it also by a Lagrangian relaxation of the searcher's nogoods kept along its path, over the matching bound, and (with `fix_forced`) force the members without which that bound would cut; the multipliers are refitted at the root as the searcher learns (section 4); with `gf2` it changes no tree and costs 2 to 4% |
| `gf2` | on | proven (docs/THEORY.md, Theorem 11; weak duality for the forced members) | bound every node by the GF(2) bound (section 4) instead of the matching bound and its refinement, branch on the conflicts of its heaviest set by exact strong branching, and (with `fix_forced`) force the members without which it would cut; the Lagrangian bound over the nogoods, if on, applies after it |
| `symmetry` | on | proven (docs/THEORY.md, Theorem 13) | cut a node whose completions all break a symmetric pair's inequality, and drop from a node's candidates those that no completion keeping the inequalities holds (section 4, "Prime exchanges") |
| `force_window` | all | sound (a member is called forced only where the exchange graph shows every completion without it short) | with `gf2` and `fix_forced`, which members of a node's heaviest set that the weight split leaves free the exchange graph searches for being forced: those whose split bound is less than this far above the need (`80`; `0`: none), less than a percentage of the node's slack above it (`25%`), or `all`; a wider window forces more members, for a smaller tree, at the cost of more searches (section 4, "Forced members", has the measurements of `all` against `80`) |
| `branching` | exact | sound (every completion leaves out some member of the conflict) | with `gf2`, how a node chooses the member of its conflict to branch on: `exact` (the bound without each member, certified or solved), `forced` (each solved only as far as the need, the branch by the split's bound), `split` (the split's bound alone), `largest`; the cheaper choices take more nodes |
| `strong_limit` | all | sound (as `branching`) | with `gf2`'s strong branching (`exact` or `forced`), the most members of a conflict whose bounds are found, the most promising by the split's bound first |
| `strong_depth` | all | sound (as `branching`) | with `gf2`, a node with at least this many promotions branches as `deep_branching` says instead of `branching` (`all`: none does), so that the deep nodes, the most numerous, can choose more cheaply |
| `deep_branching` | split | sound (as `branching`) | the choice of `branching` at the nodes `strong_depth` names |
| `more_forced` | on | sound (as `fix_forced`) | with `gf2`'s strong branching and `fix_forced`: once a member is found forced, the bounds of the other members are still found, to find more forced members |
| `prev_cap` | on | proven, given an optimal game of n − 1 | stop as soon as the incumbent reaches the ceiling, n + score(n − 1) − in_the_bag as a promotion sum (the Java's `FrameSolver.getMaxPromotionSum`) |
| `seed_incumbent` | on | sound (any promotion set is a valid incumbent) | start from the best of the greedy set and the promotions of the game of n − 1 that are still promotable |

**The frame model's own rules are not tactics.**  They define the search space, and cannot be switched off inside the
model: the greedy removal of impossible sources inside frames (the Java's `Hive.removeImpossible`); the mandatory moves
(its `Hive.unzip`); freedoms = factors − sources (its `freeFactorCount`, set by `Hive.finishSetup`); closed frames
taking no part in the search; and a promoted number paying tax only from the frame one forward.  Section 2 ("What
optimal means") says what is proven of them, and what remains: statement U.

**Where the tactics are recorded.**  A result line records the tactics it was made with: `tactics`, the changes from
the defaults (such as `lagrange=on,gf2=off`), and `tactic_switches`, every tactic's value.  A line of a build before
`gf2` has no such key, and ran the matching path.  A checkpoint records the search tactics, and is refused when they
differ: the error names each tactic that differs, and the `--tactic` flags that resume it.

*The tactics removed.*  The [notebook](../notebook/README.md) has what each tactic was worth and the order they were
found in, and the tactics that were removed: the Java's two conjectures, `reuse_prev` (accept the previous game's
promotions with no search) and `revise_freedoms` (lower frame 0's freedoms by solving the two-frame game, assuming that
the heaviest promotion set also has the most members, which is false at n = 833); and the tactics that lost, the
Java's `blameGame` bound with its runner-up, its preclusion table, and the branchings other than strong branching
([which tactics earn their keep](../notebook/which-tactics-earn-their-keep-2026-10/README.md)).

## 6. Engine

The engine (`engine`) is a parallel depth-first branch-and-bound driver that knows nothing of Taxman: the Taxman search
is a `Searcher` that it runs.  The module comment of `src/engine.rs` has its protocol (conservation, splitting, spill,
checkpoint, termination), with the types (`Unit`, `Searcher`, `Ctx`, `EngineConfig`, `run`) next to it; this section is
the design in outline.

* **Units and workers.**  The caller's problem is a search tree whose subtrees are *units*, serialized by the `Unit`
  trait (for Taxman, two bitsets).  The engine runs `threads` OS threads (std only), each with its own `Searcher` made
  on that thread, so per-thread caches need no locks and, on a NUMA machine, each searcher's state is local.  The
  threads explore units depth first.  A searcher calls `ctx.poll()` at least every 1,000 nodes or so; when it answers
  `Yield`, the searcher pushes all of its unexplored work as units, shallowest first, and returns.
* **Work distribution.**  One shared deque holds the units, shallow at the front and deep at the back.  A worker that
  yields pushes its stack's remaining work and then pops from the back, resuming its own deepest work; idle workers take
  from the front, which holds the largest subtrees.  A worker yields when idle workers are left that the queued units
  cannot serve (a *split*), when a checkpoint is due, or when a stop is requested.  With one thread the deque is a pure
  stack: units are explored in exact depth-first order, whatever the checkpoints and the spilling.  The engine keeps one
  open split request per idle worker, and any running worker can claim one with an atomic decrement, so as many workers
  split at once as there are idle workers to feed.  A worker answers only after 64 polls of its unit, so that a fresh
  unit first gets deep enough to have something to give.  One locked deque is enough, because units exist only after
  yields and a rebuild costs the searcher far more than the lock ([the engine](../notebook/the-engine/README.md)).
* **Memory bound.**  The in-memory deque holds at most `queue_mem_cap` units; beyond that the engine spills the oldest
  to segment files in `spill_dir`, and reloads them when memory runs dry.  Searchers keep only their stack and bounded
  caches.
* **Incumbent.**  An `AtomicU64` value with a payload behind a mutex (the promotion set, encoded).  Offers only ever
  raise it.  A searcher can declare it optimal, which ends the run, as an incumbent that reaches the ceiling given
  through the control (below) does.
* **Checkpoint.**  Every `checkpoint_interval` all workers yield and park.  The engine writes the caller's header, the
  incumbent, statistics and every unit, in memory and spilled, to `<checkpoint>.tmp`, fsyncs it, renames it over the
  checkpoint, syncs the directory, and releases the workers.  Resuming loads the file and goes on, losing and
  duplicating nothing beyond partly explored nodes.  Any interval works, zero included.  The caller's header identifies
  the search, so that a checkpoint of another game, width, build or tactics refuses to resume (section 8).  A run that
  completes deletes its checkpoint, unless the caller asks to keep it until it has recorded the result.
* **Stopping and sharing.**  Time and node limits, and SIGINT and SIGTERM (after `stop_on_signals`), stop a run with a
  final checkpoint; a second signal kills the process.  A stop that already holds when a run starts (a time limit of
  zero, a signal received before) does no search work, however the threads are scheduled: the run checks before it
  starts a worker (engine docs, "Starting stopped").  `taxman solve` takes an exclusive `flock` on `taxman.lock` in its
  checkpoint directory, so a second process there fails at once.  Spill segments carry their process id, and a run
  deletes those of processes that no longer exist.  Progress lines and results on stdout ignore write errors, so a
  closed pipe cannot kill a run.  A panic in a release build aborts (`panic = "abort"`), and the previous checkpoint
  stays valid, which is the property that matters.
* **Control** (`engine::Control`).  Another thread steers a run through this handle while it goes: a ceiling (a proven
  upper bound, which ends the run once the incumbent reaches it), outside incumbents, a stop, and the number of
  workers.  The number grows by starting workers and shrinks by retiring workers that hold no unit (a running one hands
  all its units to the deque first); 0 suspends the search, its units waiting in the deque until the number rises
  again.  Requests made before the run apply when it starts (engine docs, "The control").
* **Instrumentation** (`engine::Counters`, per invocation).  The engine measures each worker's time, split into busy,
  idle, parked and waiting for the lock; the effective parallelism, busy time over capacity (the workers' lifetimes
  summed, the wall time times the threads when their number is fixed); splits and the units they gave, steals, runs
  and tiny runs; when all workers first ran at once; the workers added and retired; the idle time of the ramp and of
  the tail; the time from a unit's start to its first step (the rebuild) and the time in that step; and the pauses and
  the longest wait for the workers to park.  Every progress line ends with them in short ([REFERENCE.md, "Reading the
  output"](REFERENCE.md#3-reading-the-output)).

## 7. Per-game pipeline (`solver`)

Each game goes through six steps, from its board's tables to its line in the results file.  The shortcuts of steps
1a and 2, and the repeat in step 4, conclude a game without a search when another game already gives its answer.
`solver::solve_game` runs the steps for one game and `solve_range` for a range; the module comment of `src/solver.rs`
has the exact steps and checks.  Each step, with the Java class it reimplements:

| step | what | Java |
|---|---|---|
| 1 | the board's tables, and the set width | `FactorTable` |
| 1a | with `use_prev` and `known.boards` (`solve --use-prev`): a game that a known source has proven (the committed chain, the bounds store, the table, in that order) is taken with no search (method `known`; section 8, "Known games") | |
| 2 | the prime shortcut: a prime n plays the game of n − 1 with its first move replaced by n (method `prime`) | `FrameSolver.solutionForPrime` |
| 3 | the frames, formed, simplified, numbered and merged; with no candidates the game with no promotions is the only one (method `trivial`) | `bombus.Apiary`, `Hive`, `frame.FrameBuilder`, `WorkingFrame` |
| 4 | the search (method `search`): the frame set and the greedy incumbent (`search::prepare`); a repeat of an earlier game's search (method `repeat`, tactic `same_search`; section 8); the starting incumbent, the ceiling and the seed; the engine; promotions → moves | `search.SearchManager.create`, `Frame`, `FrameSet`, `LowerBoundEstimator.greedy`, `BranchExplorer`; `Apiary(board, promotions).getSolution()` |
| 5 | `conclude`: replay and check the game, confirm its score against `optimal.json`, assign its proof and what it assumes | `game.Solution.verify` |
| 6 | record the result (a range) | |

The steps again, with the functions that take them:

```
 taxman solve A-B --out FILE --checkpoint-dir DIR
   |  solver::solve_range_each        n = A..B, concluded in order (with --window K up to K games in flight, each set
   |                                  up and searched on threads of its own: solver/window.rs); the game of n-1 comes
   |                                  from this run, FILE or (--use-prev) the known games (known::Known),
   |                                  and is checked (legal, over, opens with the largest prime) before anything uses it
   v
 solver::set_up(n) -> set_up_with_width::<W>      W = 8 to 128 words of 64 bits (bits::WIDTHS), by n
   |
   |  [1] numtheory::Tables            maximal factors and maximal composites of 1..=n; the set width W
   |  [2] n prime ............ accelerate::prime_shortcut   the game of n-1 with its first move replaced by n
   |                                                         method=prime        proof=prime_formula
   |  [3] board::Board::new            frames::BoardFrames: sources (never tax) + factors -> frames; unzip,
   |                                   removeImpossible
   |                                   frames::build_frames: frames with freedoms numbered from the front (0 = front)
   |                                   and merged; candidates; in_the_bag
   |      no candidates ...... the game with no promotions   method=trivial      proof=trivial
   |  [4] search ............. method=search                 proof=exhausted | cap
   |        Board::prepare             search::prepare: model::FrameGraph::build -> the root FrameSet; greedy set;
   |                                   max_total; the search::Problem it poses, and its digest
   |        same_search ...... one of the 8 games before it posed the same problem and settled it: its promotions
   |                                                         method=repeat       proof=same_search
   |        search::search             starting incumbent = best of greedy / seed (n-1's promotions) / rounded root
   |                                   matching + local search; ceiling = n + score(n-1) - in_the_bag
   |          engine::run              worker threads; shared deque with disk spill; incumbent; checkpoints; signals
   |            Stepper::step          per node (search/searcher.rs): check_node = the rule of symmetric pairs
   |                                   (symmetry, search::symmetry) -> GF(2) bound (gf2; with gf2=off
   |                                   the matching bound, matchbound, and its refinement) -> conflicts, forced
   |                                   members, nogoods (search::refine, model::Nogoods) -> with lagrange the
   |                                   Lagrangian bound over the nogoods (search::lagrange);
   |                                   choose_branch (a candidate) -> make_child -> check_child
   |      promotions -> accelerate::solution_for_promotions   (frames::BoardFrames with the promotions as sources)
   |                                                         -> ordered moves
   |
   |  [2] and same_search are skipped by --force-search and when the game has a checkpoint (a search is resumed,
   |  never replaced)
   v
 [5] solver::conclude                replay (game::replay_finished); score == in_the_bag + promotions; the ceilings;
   |                                 confirm the score against optimal.json; the proof and what it assumes
   v
 [6] results::ResultsFile::append    the JSON line, synced; only then is the game's checkpoint deleted;
                                     the game becomes the previous game of n+1
```

*The shortcuts.*  Step 2 needs the game of n − 1, and the repeat of step 4 an earlier game.  `force_search` skips both,
and step 1a, and so does a checkpoint of the game: a search that was interrupted is resumed, never replaced by a
derivation.  With `use_prev`, the search of step 4 takes two things from the bounds store for a game the store has not
proven: the game of its best lower bound, a seed beside the game of n − 1 (the search takes the one that keeps more),
and its best upper bound, a ceiling beside the one that the game of n − 1 gives (the lower of the two stops the
search).  `force_search` leaves the store out.

*The game of n − 1* is checked before anything uses it: it must replay legally to its recorded score, be over, and open
with the largest prime (`conclude` has already checked a game that this run made).  The results whose proofs rest on it
assume what it assumes (section 8).

*Limits.*  A search that a time or node limit stops gives an incomplete result (`completed` false, `best_so_far` the
best game so far) and keeps its checkpoint, and a range stops there (but see "Past limits", section 8).

*Range runs* pass each game on to the next as its game of n − 1, and resume from the results file.  The solver checks
the file when it opens it: it replays every game, and checks the chain from n − 1 and each line's arithmetic and
provenance (the search problems that the checks of repeats set up again are set up first, on the run's threads).  A
range skips the games the file completed.  It appends each result and syncs it as soon as the game is concluded, in the
order of n, and only then deletes the game's checkpoint; a checkpoint left for a game that the file has completed is
deleted when a range starts.  The game of n − 1 comes from, in order: this run, the results file, and with `use_prev`
(`--use-prev`) the known games (section 8, "Known games": the committed chain's, the bounds store's, `optimal.json`'s).
With none, the game is solved *standalone*, with no seed and no ceiling.

### The window: games in flight

A range keeps several consecutive games in flight, so that a machine of many threads stays busy where one game's search
cannot use them all.  A game of the chain needs two things of the game before it: its moves, for the seed, and its
score, for the ceiling.  Everything else (its board, its frames, its search problem, whether an earlier game settled
that problem) is its own, and the ceiling only stops a search early.  So a range keeps a *window* of K consecutive games
in flight (`--window K`, `SolveOptions::window`; 4 by default, as in `solver/tools/cloud/chain.sh`, and always 1 with
one thread).  Game n + 1 is set up while n searches.  Its search may start before n is proven, *speculatively*, seeded
from n's best game so far and without a ceiling, and it takes n's ceiling and the seed of n's final game the moment n is
concluded, through its engine's control (section 6).  `solver::window` is the driver.  With K = 1 it is the range solved
in order, line for line: a game enters the window only once the game before it is concluded, so its search starts with
the final seed and the ceiling, on every thread.

* *Phases.*  A game is set up on a thread of its own (the prime shortcut, the trivial game, or the search problem).
  The driver looks its repeat up on its own thread, in the order of n; a game of the window that has set a search up
  counts (section 8).  The game's search runs on its own thread.  The driver concludes the game when it is the oldest
  game of the window and the game before it is concluded: it builds and checks the result (`conclude`), appends and
  syncs it, and only then tells the next game its ceiling.  A repeat waits for the game it repeats.
* *Seeds.*  A speculative search starts once the game before it has a game to seed it with: the concluded game, its
  search's best so far (a search offers its start to its control as the start is made; section 4), or what that gives
  through a repeat or the prime formula.  Every 5 ms the driver offers it the survivors of that game again whenever its
  score has improved, and at the conclusion those of the final game.
* *Deferring.*  A search whose seed's survivors reach the *provisional ceiling*, n plus the score of the best game so
  far of the game before, less `in_the_bag`, waits for that game's conclusion: if that best is final, the ceiling
  settles the game at once, and a speculative search of it would be wasted.
* *Threads.*  `--threads T` is a budget that the window shares (`solver::window::allot`, a pure policy, unit-tested).
  A setup holds one thread, and a search the workers of its engine, which takes on workers and retires them while it
  runs (`Control::set_workers`; 0 suspends it, its units waiting in its deque).  Every 5 ms the driver samples each
  engine and serves the claims, oldest game first:
  * a setup asks for one thread;
  * a search ready to start asks for every free thread if it is the only claim, and for one otherwise;
  * a busy search (idle under 5% of the last tick) asks to double;
  * a search idle 25% or more gives its idle workers back to another game's claim, unless it was given threads in the
    last 10 ms;
  * and the oldest search, if it is left short, reclaims threads from the younger ones, youngest first, suspending
    them if need be.

  So the game the chain waits for runs on every thread it keeps busy, speculative searches run on what it leaves, and
  the threads of a search that ends or idles go to the oldest game that can use them.  The driver does not estimate
  the tree: a search earns threads by keeping them busy, the online form of a running estimate of what is left of it.
* *Stops.*  The driver reports a failure, and ends the range at an unfinished search, only when its game is the
  oldest, so a run fails or stops at the game, and with the lines before it, where a range solved in order would.  It
  then stops the searches still in flight with a final checkpoint (the next run resumes them), and drops what they
  found.
* *The wall.*  `--game-hours H` (`SolveOptions::game_limit`) ends the range at the first game whose own search passes H
  hours, to find the game at which a machine's day is not enough.  A game's own search is the time it has been the
  oldest game of the window with its search running, summed over the invocations that searched it.  It is not the time
  the game waited behind an older one, which its checkpoint's elapsed time counts: a younger search is suspended while
  the oldest keeps the threads busy, so a game that follows a dear one would arrive at its own search with that time
  already on its clock.  The driver keeps the clock in `nNNNN.clock` beside the checkpoint, every checkpoint interval
  and when the search stops, and reads it back when a search starts from a checkpoint.  A game that restarts from its
  beginning starts its clock again, and a finished game's clock goes with its checkpoint.  At the limit the range ends
  as it does at a stop signal, every search in flight stopped with a final checkpoint, but records nothing for the
  game, not even an incomplete line; `solve` prints `wall at N after H hours` and exits with the status 4
  (`RangeSummary::walled`).  Run again with a larger limit, the game goes on from its checkpoint and its clock.
* *Trace.*  With `TAXMAN_WINDOW_TRACE` set, the driver prints its events on stderr: setups, the starts of searches with
  their seeds and ceilings, workers given and taken back, hints, deferrals and conclusions.

[Round B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md) has what the window buys and what was tried
on the way.

**The table of optimal games is built into the binary** (`optimal.json`, through `include_str!` and
`game::Optimal::built_in`): the table as it was when the crate was compiled, which a rebuild picks up when the file
changes.  The solver confirms every completed game's score against it, and a score that differs is a hard error that
names the game.  No option names another table, so a run cannot be checked against anything else; the library takes
another table where a test needs one (`solver::solve_range_each_with`, `results::read_results`).

## 8. Results and the trust model

Every completed game records how its optimality is proven, its *proof*, and what that proof takes as given beyond the
frame model, what it *assumes*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file.  [REFERENCE.md, "Why the answers can be
trusted"](REFERENCE.md#4-why-the-answers-can-be-trusted), has the proof kinds, the assumptions and the verifiers as a
results line shows them, and section 2 says what each kind of proof needs of the frame model.  This section is the
design behind them; `src/results.rs` has the full rules and the checks.

**What a game assumes** is what its proof rests on beyond the frame model, inherited down the chain: a proof that rests
on the game of n − 1 assumes what that game assumes, a repeat what the game it repeats assumes, and a `known` proof what
its source's game assumes.  Two sources add assumptions.  A game taken from `optimal.json` (`--use-prev`) is assumed
optimal, since the table records nothing about how a game was found: a proof that rests on its game of n − 1 assumes
"the game of n − 1 in optimal.json is optimal", and a game of n taken from it "the game of n in optimal.json is
optimal".  A bound of the bounds store holds wherever statement U holds at the games it rests on, so a game that rests
on it assumes "statement U holds at n=B" for each such game B.  A game that assumes nothing is proven within the frame
model, given that the solver is right, and a chain from n = 1 assumes nothing.  The solver refuses no game for what it
assumes: the line says it, and the verifiers count it.

**Checks and referees.**  Every game is replayed by `game::replay_finished` before it counts, and checked against its
recorded arithmetic and the ceilings `score(n−1) ≤ score(n) ≤ n + score(n−1)`; a game that fails after it was found goes
to `<results>.rejected.jsonl`, and the run fails.  `taxman verify-results` reads results files without changing them,
and runs the same checks and the agreement between files.  `solver/tools/check_results.py` is an independent referee in
Python: it derives every proof and assumption again, and compares the scores with `optimal.json`, the Franklín–Moniot
upper bound and the best-known heuristic chain.  Both read results lines of schema 5 only.  Both refuse a line whose
fields contradict each other (say a sum at the ceiling without `reached_cap`), since they derive the proof and the
assumptions from the fields.  A line that cites a known source is checked against the sources, which both read as they
are (`known::Known::check_cite`; the Python referee from the lines of the files).  No check can catch a game that is
legal, finished and within the ceilings but not optimal, when the game of n − 1 is not optimal either: that is what the
proof kinds and the assumptions track, and why a chain from n = 1 matters.  [results/README.md](../../results/README.md)
says what supports each committed score.

**Result lines** are one JSON object per line ([REFERENCE.md, "A results line
(`--out`, schema 5)"](REFERENCE.md#a-results-line---out-schema-5)).  A line is never rewritten, and a run locks the
results file it appends to.

**What identifies a checkpoint.**  The header that a search stores identifies the search, so that a build never resumes
a checkpoint of another search.  It holds:

* a format version, the set width and n;
* digests of the root's candidates, of the frames (every frame's moves, factors and freedoms, as built) and of the
  one-sided candidates of the extended problem (section 4, "Extended mode");
* `max_total` and the search tactics, and from format version 6 the tactics that tune the GF(2) search (`force_window`
  to `more_forced`, section 5).

A search tuned as the builds before those tactics were (`force_window=80` and the other five at their defaults) writes
version 5, which implies that tuning, so its checkpoint is the one those builds wrote; a search tuned otherwise writes
version 6.  The default's window is `all`, so the default's search writes version 6, and resumes a checkpoint of
version 5 (or 4) only as `--tactic force_window=80`, as for any tactic that differs; a build from before that change
reads a checkpoint of version 6 as the search it is.  The tactics that set how the nogoods are kept (`nogood_period`,
`nogood_keep` and `nogood_limit`) are in no version: they change the trees and never the answer, and a resumed search
starts with empty tables, so the run that resumes says which it uses.

This build also reads version 4, the format of the builds before the extended problem's search, whose headers lack the
digest of the one-sided candidates.  Those builds searched no problem with one-sided candidates, so a version-4 header
reads as version 5 with the digest of none: every other field is checked as for version 5, the run goes on under the
header it was written with (the engine compares it byte for byte), and a version-4 checkpoint asked to resume a search
with one-sided candidates is refused, saying so.  So a chain stopped by a build before the extended problem's search
resumes; `tests/wide.rs` resumes a real version-4 checkpoint to the node count of the build that wrote it.  Every other
version is refused with one message, so a checkpoint of an older build cannot resume with tactics that are gone.

Resuming checks all of these and says which differs.  It also checks that the stored incumbent is a set of candidates
that can be promoted from the root and is worth the stored value.  So a checkpoint of another game, or of an older build
whose frame builder differs, is refused, instead of pruning with a foreign incumbent or failing to rebuild its units.  A
resumed game rests on nothing that a game run in one piece does not.

### What a window changes in the results

A range solved with a window of games in flight (section 7) records what the range solved in order records, but for
what a multi-threaded search may vary:

* A line is appended only after every line it rests on: that of n − 1 (its `prev`) and, for a repeat, that of the game
  m it repeats.  Both are earlier lines, and lines are appended in the order of n.
* `prev`, `same_search` and `assumes` are derived as in order, from concluded games.  A repeat names the same m: game n
  looks among the eight games before it for the latest game whose problem is n's, counting a game of the window that has
  set a search up and is not concluded yet.  In order, that game is concluded when n looks, and every search or repeat
  that completes settles its problem, while one that does not complete stops the range, or fails it, before n's line
  could be written.
* The proof kinds are those of the range solved in order.  A completed search's promotion sum is its problem's best,
  P\*, and its proof is `cap` exactly when P\* reaches the ceiling, however the search ran.  A search started before its
  ceiling was known may have explored its whole tree to an optimal set at the ceiling: its line records `cap` and
  `reached_cap`, the weaker of the two proofs it has, as the verifiers' rule wants (a line reaches the ceiling exactly
  when its promotions sum to it).  One stopped by a signal or a limit with its best at the ceiling is complete by the
  ceiling, as it would have stopped there given the ceiling from the start.
* A seed from a provisional game, a hint offered while a search runs, a ceiling given late, a deferral and a suspension
  change how a search runs, never what it proves.  Every set offered is promotable (the survivors of a seed), a deferral
  only delays a search, a suspended search keeps every unit, and the ceiling is proven given the game of n − 1 optimal,
  which a proof by `cap` records that it rests on.
* What may differ: the moves and promotions where a problem has several optimal sets (as between thread counts), and
  with them the next line's `prev.moves_digest`; the nodes, the search's counters, the engine's, and the times.

### Past limits

With `--keep-going`, a game whose search a limit stops is recorded as an unfinished line (its best game so far, no
proof), and the range goes on (`solver::SolveOptions::keep_going`; the driver is `window`, "Past limits").  The next
game may take from that game only what does not rest on its being optimal.  A *seed* is any promotable set the board can
use, offered as an incumbent, so it cannot change what a search proves.  Three things do rest on opt(n − 1): the ceiling
`n + opt(n − 1)` (a `cap` proof), the prime formula (proved at `accelerate::prime_shortcut`), and the tripwire that a
score above that ceiling proves the game before it not optimal.  The solver takes each of them from a game only if
`Prev::proven`, which is true of a completed game and of `optimal.json`'s (assumed), and false of the best game of a
stopped search.  A repeat rests on the earlier game it repeats, which is a completed game of the file or the range,
never an unfinished one.  The check that a completed game scores no less than the game before it stays, since a game of
n − 1 is a game of n, optimal or not.

The lines say what a game had: a game built on an unfinished one has `prev` with no proof, `setup.cap` null and no `cap`
or `prime_formula` proof, and both referees refuse a line that claims one of these on it ([REFERENCE.md, "What a game
had from n - 1"](REFERENCE.md#what-a-game-had-from-n---1)).  The format of a line is the same, except that `options`
also names `keep_going`, `retry`, `retry_improved` and `pass` where they were given.  The games of such a range are
solved one at a time, because a game that went on from a game in flight would not know whether to take its ceiling.

### Known games

`--use-prev` draws on the games that other work has made (`src/known.rs`): `optimal.json`, the committed chain
`results/strict.jsonl`, and the proven games of the bounds store `results/bounds.jsonl` (`src/store.rs`; the ledger
writes it).  They are the repository's `results` (`paths`), and no option names another.  `Known::game(n)` is the game
of n from the best source that holds one: the committed chain's, which is as proven as its line and assumes what the
line does, then the store's, then the table's.  A game of the store is *proven* when its best lower bound equals its
best upper bound.  Its game is the lower bound's, played by the referee at the value (a line that cites the game of
another line is carried from it, `store::carried`), and what it rests on is the upper line's `rests_on`.  In the
options, `SolveOptions::use_prev` turns it on and `SolveOptions::known` names the files and how far it goes; `regress`
sets only the first, so that it takes just the game before its range, from the table.

* *The game before the range* (`previous_game`) is the results file's, else the known one.  It gives the ceiling, the
  seed and the prime formula as any proven game of n − 1 does, and a line built on it says where it came from in
  `prev.source` (`optimal`, `strict` or `bounds`), with that game's proof and assumptions.
* *A known game* (`Derived::Known`, step 1a of the pipeline, `known.boards`) is recorded without a search.  Its line
  has the method `known`, the proof `known`, no `setup` and no `in_the_bag` (it set no board up), and `known`: the
  citation.  Its promotions are the board's promotion candidates that the game plays, as a prime game's are.
  `conclude` checks it like any game (the replay, the score the source gave, the chain bounds against n − 1, the
  table), and it becomes the game of n − 1 of the next game, whose proofs that rest on n − 1 inherit its assumptions.
  It sets up no search problem, so no later game repeats it, unless its proof is the store's bound reached by a search
  (below).
* *An open game of the store* lends its lower bound's game as a seed, and its upper bound as a second ceiling
  (`StoreCap`, `search_cap`: the engine is given the lower of it and the ceiling of the game before, at the start and
  when the game before is concluded).  A search whose best promotion sum reaches the store's bound, when it is not the
  chain's ceiling that it reached, is proven by the bound: proof `known`, method `search`, `known` citing the upper
  line alone, `reached_cap` false and `setup.cap` as for any search.  Such a game settles its search problem, as an
  exhausted search or one at the ceiling does: its promotions are a promotable set of that sum, and no promotable set
  weighs more than the bound allows (a game of the frame model is a game of the board, so it scores at most opt(n),
  which the bound bounds).  So a later game that poses the same problem repeats it, and assumes what the bound does.  A
  search that finds a game above the store's bound fails the run at that game: the store is wrong, or statement U fails
  at a game that its bound rests on.
* *`force_search`* leaves out the known games and the store's help, since a forced search is the chain's own.  A game
  with a checkpoint is resumed, never replaced.

The referees (`Known::check_cite`, and its Python counterpart in `solver/tools/check_results.py`) check a citation: the
table has a game of n with the line's score; so has the committed chain; the store has a lower line and an upper line
of game n worth the score, of the rule and from the run that the citation names (a store has one line of a game, side
and value).  They take the sources as they are: the referees of results files check the committed chain, and `ledger
verify-bounds` and `solver/tools/check_bounds.py` check the store.  What the citation lets them conclude is that the
game, a finished game of that score, is optimal where the source's bounds hold; they do not compare which game the line
plays with the source's.  The assumptions are those of the cited lines (`assumed_optimal`, the committed line's, and
`assumed_u` for each game of the upper line's `rests_on`), and the referees refuse a line that records others.  Results
lines stay in schema 5: `known` is a key that only such a line has, a method and a proof, and `strict` and `bounds` are
sources of `prev`.  Every older file reads as it did, and an older build refuses a file with a `known` line, whose
proof it does not know.

### The ledger

The ledger proves many games at once by bounding each from both sides: for every game of a stretch it keeps a lower
bound L(n), the score of a finished game that the referee replays, and an upper bound U(n), each with its derivation,
and it proves a game when they meet.  It is `examples/ledger/`, whose module comment has the commands and the files.  It
reads nothing of the truth (`optimal.json`, the results files) unless it is given proven games.

*Its sources of bounds:*

* the *map* of each game: a search stopped before its first node, which gives the start incumbent and the root's GF(2)
  bound;
* limited one-thread searches, each resumed from its checkpoint, seeded with the games of n − 1, n and n + 1, and with
  U(n) as its ceiling.  A stopped search's upper bound is its *dual bound* (`search::dual_bound`): the largest of its
  incumbent and the bounds of the open units of its final checkpoint, each the GF(2) bound of the node as the search
  computes it when it starts the unit (`search::NodeBound`);
* given games;
* the relations between games, applied to a fixpoint after every change: monotonicity, the ceiling and the game below
  it, the prime formula both ways, and repeats (docs/THEORY.md, "The ledger: bounds between boards", Lemmas L1 to L4
  and D, with what each rests on).  The referee replays every game that a relation makes before it counts, if it could
  raise a game's L (a game that cannot is not shown to the referee, or not made), and an L above a U stops the run as
  an error;
* the *finder*, a large neighbourhood search (`examples/common/lns.rs`; [finders for good games that are not
  sound](../notebook/finders-for-good-games-that-are-not-sound-2026-10/README.md)).  It is not sound and proves
  nothing, but a lower bound needs only a legal game.  It runs for `--lns-seconds` seconds of a worker, after a game's
  map and before a search whose best seed has changed since it last ran on the game (a climb's always has).  Its game,
  which the referee replays like every lower bound, is offered as the game's L, and seeds the search if it is better.

*Its order.*  The maps of all the games come first, on the workers, and the relations carry their bounds once, when the
last map is in; the main thread, which applies every result while the workers whose searches ended wait for it, does
little for each.  Then the searches go where a proof looks cheapest, with *climbs* ahead of them:

* a game whose game below its repeat class is proven, however it was proven, is searched next, for a limit of its own
  (`--climb-limit`), seeded with that game and capped by the ceiling it gives; when a climb proves its game, the game
  above its class is climbed to (a climb up);
* after those, a game whose game above its repeat class is proven is climbed to downward, seeded with that game but
  under the ledger's own, loose ceiling (`--climb` picks the ways);
* a climb that stops parks its game in the ordinary order, and the games just outside the class of a game being
  climbed to wait for it;
* an open game that is not climbed to is *stepped down* before its proof (`--floor-step`): a search under a floor below
  its upper bound (`search::search_above`, Lemma F) which, if it completes having found nothing, makes the floor the
  upper bound (the rule `floor`).

The cost of a search under a floor is about log-linear in the floor's distance below the root's bound, so each game has
a line, fitted from its completed steps, which forecasts its proof.  Each step costs tenfold more than the depth already
proven.  A game goes to its proof (the ordinary search, seeded and capped) when its gap is within two steps.  After the
climbs and the games that a proven neighbour has changed, the games go by the forecast seconds of their proofs, cheapest
first (`examples/ledger/cost.rs`; with no floor steps, by least `level + gap / gap_unit`, which finds the games proven
with no help from below).

A proven game of the ledger is proven within the frame model, as a game of the chain past 2175 is, given U at the games
its upper bound came through.  The ledger is not a results file, and nothing of it goes into `results/strict.jsonl`.

**The bounds store** keeps the bounds that a run finds for good: `results/bounds.jsonl`, one JSON line a bound, in
schema 1, appended and never changed.  A line is added only when it improves the file's best bound of its game and side,
and none for a game that the chain has proven.  `src/store.rs` has the format and the relations between games, which
`taxman solve --use-prev` reads too ("Known games", above); `examples/ledger/store.rs` has the ledger's checks of a line
against the lines before it; and [results/README.md, "bounds.jsonl"](../../results/README.md#boundsjsonl) has the
format, the commands and the referees.

* A lower bound is a game that the referee replays at the line's value: its promotions where it was made from them,
  which `solution_for_promotions` turns into moves; cited (`from` and `from_value`) where a relation made it from the
  game of another line; else its moves.
* An upper bound has its rule, the bound and game it was derived from (`from`, `from_value`, and for a repeat the two
  bags), and `rests_on`, the games at which it assumes statement U: its own game for a bound of the frame model, what
  the bound it comes from rests on for a relation, and its own game added for a repeat (Lemma L4's inequality between
  upper bounds).
* `ledger export` appends a run's improvements, and `ledger run --bounds` starts a run from the file's best bounds (a
  game with both needs no map).  `ledger verify-bounds` is the Rust referee: it replays every game, computes again every
  root bound and every repeat's bags and search problem, and checks every relation's arithmetic against the line it
  cites.  `solver/tools/check_bounds.py` is the Python one: it replays the games, the cited ones rebuilt by its own
  constructions and promotion sets from the moves that `ledger games` writes, and checks the arithmetic.

The store holds only bounds that hold: the finder's games are in it because the referee has replayed them.

### Same search

Moving from one board to the next often leaves the search untouched.  The new number lands in the part of the game that
setup settles.  The frames that the search sees, and their candidates, stay as they were.  The game then poses exactly
the same search problem as an earlier one, so it has the same best promotions.  The tactic `same_search` (proven, on by
default) recognizes this and reuses the earlier game's promotions instead of searching again.  About half of the
committed chain's games that need a search are such repeats ([results/README.md](../../results/README.md#strictjsonl)
has the counts).  Skipping their searches saves a large share of a chain's search time; [the
entry](../notebook/same-search-2026-10/README.md) has the measurements.

*The search problem.*  A game's *search problem* Π is what setup gives the search, less what only changes how the search
runs.  It has four parts:

* the model: the factor graph among the frames' numbers, and each number's home frame;
* the root: every frame's moves, factors and freedoms;
* the candidates;
* `max_total`, the most promotions the frames' freedoms allow.

The searcher also gets the tactics, the ceiling and the seed (`search::prepare`).  They change how the search runs, not
what it finds (section 4).  The greedy set and the matching graph are functions of Π.  The best promotion sum P\*(Π) is
the largest sum of a *promotable* set of Π: a set of at most `max_total` candidates that can be promoted from the root,
keeping every frame winnable and its freedoms at least 0.

Which sets are promotable depends only on the frames and on the graph's edges among their numbers.  Promotion and
reduction see only the numbers in the frames.  An edge is the maximal-factor relation (y = x/p, p prime), a property of
the two numbers that does not depend on n.

The type `search::Problem` writes Π out canonically, as numbers, so that the form does not depend on the set width.  It
lists the frames front first, each with its freedoms, moves and factors; then the candidates and `max_total`; then, for
every number of the frames, its home frame and the lists of maximal factors and maximal composites that the model's
graph keeps for it.  Those lists name some numbers that reduction removed and the search never sees, which errs on the
side of telling problems apart.  Equal forms are equal problems, with the same promotable sets and the same P\*.  A
results line records the form's 64-bit digest (`setup.digest`), but nothing rests on the digest alone ("Finding m",
below).

*Which proofs settle P\*.*  A repeat is only as good as the earlier game's proof.  Three kinds of proof of a game m
establish P\*(Π_m):

* `exhausted`: the search explored the whole tree of Π_m.  So the sum of m's promotions is P\*(Π_m), given that the
  pruning is exact, which every exhausted proof rests on.
* `cap`: m's promotions S are promotable and sum to ceiling(m) = m + score(m − 1) − in_the_bag(m), the ceiling as a
  promotion sum.  So P\*(Π_m) ≥ ceiling(m).  Conversely, every promotable set T of Π_m is a legal game of m scoring
  in_the_bag(m) + sum(T).  This is the soundness of the frame model (THEORY.md, Theorem M).  Every search relies on it
  each time it offers an incumbent, and the referee replays every game built from one; none has ever failed.  So
  in_the_bag(m) + P\*(Π_m) ≤ opt(m) ≤ m + opt(m − 1) = m + score(m − 1).  The last step needs the game of m − 1 to be
  optimal: the proof by the ceiling rests on that, and m's assumptions record it.  So P\*(Π_m) = ceiling(m), the sum of
  m's promotions, whatever tactics shaped the search.
* `same_search`, by induction: m repeats a game k with Π_k = Π_m whose proof settles P\*(Π_k), and m's promotions are
  k's.

No other proof settles P\*.  A trivial game has no candidates and sets no search up.  A prime game is the prime
formula's and sets none up either.  Neither is ever repeated, and the verifiers refuse a repeat that names one.  An
unfinished search settles nothing.

*The repeat.*  Suppose game n poses Π_n = Π_m, and m's proof settles it.  Then P\*(Π_n) = P\*(Π_m) = P, the sum of m's
promotions S, and S is promotable in Π_n.  The game of n is S played on the board of n (`solution_for_promotions`).  It
goes through `conclude` like any game.  It must replay legally, be over, and open with the largest prime.  It must score
in_the_bag(n) + P, stay within `score(n − 1) ≤ score(n) ≤ n + score(n − 1)`, and match optimal.json where the table has
n.

The game is optimal on the premise of an exhausted proof of n: that the best game of n scores in_the_bag(n) + P\*(Π_n),
which is U at n (section 2).  Only P\*(Π_n) is established differently, by m's proof instead of a search.  So the proof
`same_search` rests on m, not on the game of n − 1, and it assumes what m assumes.

*Finding m.*  Before it searches game n, the solver looks for an earlier game whose problem is Π_n.  It looks among the
completed games of the results file and of the run, in the `SAME_SEARCH_WINDOW` = 8 games before n, latest first,
for one that set a search up and whose proof settles it.  It does not look when the search resumes from a checkpoint, or
with `force_search`.

The digest finds candidates; the whole problem decides.  A game whose recorded digest differs is passed over.  For one
whose digest agrees, the solver sets its problem up again from its n, which takes milliseconds
(`board::search_problem`), and compares it with Π_n whole.  When the run set that board up itself, the solver takes the
problem from memory instead: the problem its setup posed, which is the same problem.  The first equal one is m.

Setting m up again with this build stands for the problem that m's own build searched.  The digest that m's build
recorded ties the two: a problem with the recorded digest is that problem, barring a 64-bit collision.  The problem set
up again must also have the sizes that m's line records: frames, candidates and `max_total`.

In the committed chain, every repeat is of the last game before it that set a search up: n − 1, or n − 2 past a prime.
So the window loses nothing there.  It also keeps to eight the setups that a range starts with, and it makes m a
function of the window's games.

*The verifiers.*  The first, `taxman verify-results`, checks a repeat's line against m's:

* m is a completed game on an earlier line of the same file, and its proof settles its search;
* the line's record of m (proof, assumptions, digest) is m's;
* the promotions are m's and sum to the recorded promotion sum;
* both problems, set up again from the two values of n, are as large as their lines record, have the recorded digest,
  and are equal;
* both games score their board's in_the_bag plus that sum.

It then derives the proof and the assumptions (m's) as for any line.

The second, `solver/tools/check_results.py`, shares no code with the solver and cannot build the model.  It checks the
same structure from the lines alone: m is earlier, completed and settles its search; the recorded digests and sizes are
equal; the promotions are equal; score − in_the_bag is equal; the assumptions are inherited.  It says that it takes the
problems' equality on trust, as it takes an exhausted search.

### Certificates of U

A *certificate* proves statement U at one game: that the recorded score of a game whose proof uses the frame model
(`exhausted`, `trivial`, `same_search`) is opt(n) on the whole board (section 2).  `taxman certify` makes certificates
game by game, by the corollary of Theorem M⁺ (THEORY.md, "The extended problem"), and `taxman verify-certificates`
checks them.  [results/README.md](../../results/README.md#certificates) says which games have one.

*What `certify` does* (`taxman certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE`;
`src/certify.rs`).  It certifies the games whose lines in the prover's barred parity bounds
(`solver/noframe/data/upper_*.jsonl`) say they need a certificate.  For such a game it takes the line's open members
O, and sets up the extended problem Π⁺(O) in extended mode (section 4, "Extended mode"), with the other members of
U\*_{≥3} fixed.  It searches that problem from the recorded game, as the promotion set (O, its moves up to n/2), with
the ceiling n + score(n − 1) − in_the_bag⁺.  Before the search it checks that the game replays to its score, plays U\*
above n/2, and is a promotion set of Π⁺(O) worth the score (Theorem M⁺ says that a playable one is).  The search ends
with one of two verdicts:

* *certified*: the search ends with nothing better, so opt(n) is the score, given that the members not open were
  certified against it.  A line of the bounds whose score is above the recorded one certifies nothing against it, and
  is refused.
* *beaten*: the search found a promotion set worth more.  Its set F ∪ K ∪ L is played in the order of the peel of
  `playable` (the whole-board oracle, which shares nothing with the frame model), finished with the largest legal move
  left until there is none, and refereed by `game::replay_finished`.  It is a counterexample to the score, and to U if
  the score rests on the frame model.  It goes to `<out>.counterexample.jsonl` as well, and the run exits with the
  status 2.

*The formulation.*  Π⁺(O) has two formulations (`Board::extended`; THEORY.md, Lemma M5⁺): with the claims of `unzip`
that the lemma allows to keep (the default), and without any (`certify --no-claims`, the formulation of every
certificate made before the lemma).  The two have the same promotion sets, and so the same heaviest promotion set (the
lemma's corollary), so a certificate made in one bounds the heaviest promotion set of the problem in the other.  A line
records the formulation it was made in (`claims`, true or false); a line with no such field reads as false.

*The same problem.*  Consecutive games often pose the same Π⁺(O) (the same `search::Problem`, which has no n in it:
"Same search"), and then an earlier game's certificate settles the later one with no search.  If game m is certified,
by a search or by this rule, and Π⁺(n) equals Π⁺(m), then the heaviest promotion set of Π⁺(n) is worth best(m) =
score(m) − in_the_bag⁺(m), which m's certificate proved, and a recorded game of n worth as much has nothing above it.

* *Where it looks.*  After the checks of n's recorded game and before its search (a checkpoint of n, if there is one,
  changes nothing), `certify` looks at the certified games of the `SAME_SEARCH_WINDOW` = 8 games before n, those of the
  file `--out` and those of the run, latest first, and never further.
* *What it compares.*  It compares the problems in the run's formulation, whatever formulation m's certificate was made
  in.  It passes over a game of the run's formulation whose line records another digest than Π⁺(n)'s, and a game of
  the other formulation (whose digest says nothing of Π⁺(n)'s) unless its frames and candidates are Π⁺(n)'s, which the
  claims do not change.  For a game that is not passed over, m's problem in the run's formulation is the one the run
  set up when it certified m (kept for the last eight games, and only if m's line is of the run's formulation).  For a
  line of the file it is set up again from m's game and the open members the line records (`certify::extended_problem`),
  in the line's own formulation, which must reproduce the line (its formulation, digest, frames, candidates, rejected
  sources and in_the_bag⁺), or the line is dropped; then, for a line of the other formulation, it is set up again in the
  run's.  `certify` compares it with Π⁺(n) whole, so that the answer never rests on two digests alone, and the first
  game with an equal problem is m.
* *What it concludes*, with worth(n) = score(n) − in_the_bag⁺(n):
  * worth(n) = best(m): n is *certified*, with `nodes` 0 and `"same_problem": {"n": m, "digest": ..., "best": ...,
    "worth": ...}` in its line;
  * worth(n) < best(m): the recorded game is not optimal, so the rule is not used and n is searched (the search finds
    the better set and says *beaten*);
  * worth(n) > best(m): impossible with genuine data (the recorded game of n is a promotion set of a problem that m's
    certificate says nothing beats), so the run fails, naming n, m and both values, and certifies nothing.

The rule is sound because equal problems have equal heaviest promotion sets: the form holds the frames, the
candidates, `max_total` and the graph's lists, and nothing of n.  The heaviest promotion set of a problem does not
depend on the formulation it was certified in, so nothing beats n's recorded game, and n's own barred bounds then give
opt(n) = score(n) by the corollary, exactly as after a search.  The refusal of a line of the bounds that was found
against a higher score applies to a game the rule settles too.

*The line.*  `--out` gets one line a game: the game, the score, the open members, how many sources greedy rejected
beside them, the frames and candidates of Π⁺(O), the nodes and seconds of the search, the threads and set width, the
formulation (`claims`), the verdict, the digest of Π⁺(O), the game it rests on if the rule settled it
(`same_problem`), and the build.  A search stopped by a limit or a signal writes an `incomplete` line and keeps its
checkpoint (the flags are `solve`'s); a rerun skips the games with a verdict and resumes the others.  A problem whose
GF(2) bound needs more block words than the engines of the game's set width have is searched at the widest set width
(128 words, five words of engine); `certify::certificate_width` is the one place that decides the width, and the line
records it.

*A list of games.*  `certify` takes the games of a list file (`--list FILE`) in place of a range `A-B`, exactly one of
the two.  The file has one n a line, in the order to certify them; it skips blank lines and those whose first non-blank
character is `#`, and has no game twice.  Any other line is a usage error that names the file and the line.  The loop
of the range takes the games in the file's order, with the same rules: the game's line of the barred bounds must exist,
a game that needs no certificate is counted (`not needing one`) and passed over, a final verdict in `--out` skips the
game, and the rule of the same problem looks at the certified games of the file and of the run, whatever order they
were made in.  The tally line reads `list FILE (K boards): ...` where a range gives `A..=B: ...`.

*The campaign's lists.*  `solver/tools/certify_plan.py` (standard library only) makes the lists of the campaign on
2176-3440, and its estimate, from the results, the barred bounds, the extended problems of
`examples/extended.rs problems` (which games the rule will settle) and the root gaps.  A game that the rule settles goes
into the list of the game it rests on, after it, since the rule reads only its own `--out`
([solver/tools/certify/](../tools/certify/README.md)).  `solver/tools/certify_campaign.py` cuts the campaign's phases
from that work list into `solver/tools/certify/lists/stage4/` (a pilot of seven dear games with a stop rule, the dear
games, four balanced lists, and the repeats the rule cannot settle), `solver/tools/certify_pilot.py` rates the pilot,
and `solver/tools/cloud/certify.sh` runs them ([RUNBOOK, section 9](RUNBOOK.md#9-certifying-u-stage-4)).

*Verifying the certificates.*  `taxman verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...]
[--threads K]` (`src/certify/verify.rs`) reads the files that `certify --out` wrote, without changing them, and checks
every line.  It notes and ignores a last line without its newline, as the file cuts it when it is opened.  A complete
line that is no certificate, or whose `build` lacks the commit (40 hexadecimal digits) or the `dirty` flag, is an
error.  It counts and skips an `incomplete` line, and checks a line with a final verdict in this order:

1. the barred bounds have a line for the game, which says it needs a certificate, and whose open members and score
   are the line's;
2. the results have a game of n with the line's score, the game passes the checks that `certify` makes of it (it
   replays to its score, plays U\* above n/2 and is a promotion set of Π⁺(O) worth the score: `check_recorded_game`,
   shared by the two), and the line's `width` is the one `certificate_width` gives; both are set up in the line's own
   formulation (`claims`);
3. Π⁺(O) set up again reproduces the line: its formulation, digest, frames, candidates, rejected sources and
   `in_the_bag⁺`;
4. a `certified` line has no `beaten_by`, and the file of counterexamples beside the certificates
   (`<certificates>.counterexample.jsonl`) has no line for its game;
5. a `certified` line with `same_problem` {m, digest, best, worth} rests on a game m among the 8 games before it, whose
   last final line before it in the same file is `certified`.  The rule of the same problem (`apply_rule`, the function
   that `certify` applies, so that the rule is written once) is applied again to the two problems, which must be equal
   whole in the formulation of the line that rests on m (m is set up again in it if m's line is of the other), and must
   give exactly the `same_problem` that the line records (the digest, `best` = score(m) − `in_the_bag⁺`(m), and `worth`
   = the line's score − `in_the_bag⁺`, equal);
6. a `beaten` line has a finished legal game (`game::replay_finished`) that scores what the line says, which is more
   than the recorded score, and the counterexample file has an identical line;
7. a game with final lines in more than one place (one file or several) has only `certified` ones: a `beaten` line
   beside another is a contradiction of verdicts.

The first line to fail ends the command with the status 1, naming the file, the line and the game.  The dear checks
(the problems set up again) run on K threads (default: all CPUs), and the first line to fail is the same for any K.
What the command cannot check is that a search explored its whole tree: it takes a `certified` line made by a search on
trust, as an exhausted proof is, and the summary says so.  The summary has, for each file, the lines, the final and
`incomplete` ones, the `certified` ones by search and by the same problem, and the `beaten` ones; then the games
certified; the counterexamples (the game, the recorded score, the score of the game that beats it); the builds that
wrote the lines (commit, `dirty`, lines); and the proven end of the chain.

*How far the proofs reach.*  A game n, from 1 to the largest game of the results, is *proven* when n = 1, or the prover
proved it, or it has a certificate that passed, or its score reaches a proven ceiling and the game of n − 1 is proven:

* the prover's proofs are the lines of `--exact` (by default every `solver/noframe/data/exact_*.jsonl`, by name; a
  later file's line for a game replaces an earlier file's, and the lines with `fix_upper` set are measurements, and
  skipped).  Their score must be the results' (a line with another score is an error naming the game and both
  scores).  A line proves its game by the rule of `exact_proofs` in `solver/noframe/tools/summarize.py`: its
  Franklín–Moniot bound or its parity bound is the score, or its search is `proven` (a search that used the segment
  cuts counts only if every smaller game is proven), or its relation is `chain` or `prime` and the game of n − 1 is
  proven;
* a ceiling is `results::CeilingProof`: the score n + score(n − 1), or the prime formula.

The command prints `proven frontier: every game to N is proven; the first that is not is M (reason)`, where the reason
is that the results have no game of M, or that it needs a certificate (the barred bounds say so) and the files given
have none.  Then it says how many games past M are proven all the same: those that no gap stops (a bound of the
prover, a certificate).  `verify::frontier` is that function of plain data (the results' scores, the prover's lines,
the games certified).  With the committed prover files and no certificate, the end is that of the prover's proofs,
whose first game past them needs a certificate ([results/README.md](../../results/README.md#what-supports-the-scores)
says how far they reach).

## 9. Testing

The tests check correctness on games that finish in seconds.  Two runs use them: the quick check, which every change
passes, and the full suite.  A reviewer of a change runs what the checklist "Before a change is merged" says; the
catalogue of the tests follows it, as details.

**Why the tests run on small games.**  The large games cannot run in a test, so the fast fixtures and the small games
stand in for them, and the tripwires and soundness tests catch the failures that no assertion on an answer sees.  The
tests run in the test profile: `opt-level = 2`, because they replay real games, with debug assertions on, which the
release profile does not have.

**The quick check and the full suite.**  The *quick check* is the command `solver/tools/quick_check.py` (it works in
`solver/frame`): `cargo fmt --check`, `cargo clippy --all-targets`, the build of the tests, and the tests of a
non-release build, which `solver/tools/cloud/run_tests.py` runs, every test a process of its own and all the CPUs
busy.  After a one-line change it takes about 12 s to rebuild and about 10 s to run the tests on a laptop of ten cores
(a plain non-release `cargo test`, one binary after another, takes about 19 s).  It verifies most merges, a refactoring
included.

**It waits for a quiet machine**, because its limits are for one with nothing else to do.  When the build has ended it
samples what everything on the machine uses of the CPUs for 2 s, and if other work uses more than 25% of the CPUs it
says so, names the busiest processes and samples again until the machine is quiet.  After 10 minutes it fails with
"machine busy" and runs no test, so that a busy machine is never taken for a slow suite.  Two checks do not run at once:
the second waits for the first's lock (a file in the system's temporary directory, which the system frees when a check
ends, however it ends) before it builds anything, says so, and counts that wait in the same 10 minutes.

**It fails**, naming the tests that are over, if the tests take more than 15 s (the target is 10 s; the build and the
wait are not counted) or if one test uses more than 2 s of CPU (CPU, not seconds on the clock, which grow several times
over when every CPU is busy).  The limits and the thresholds of the wait are the constants at the top of the script, and
raising a limit needs Brian's approval: a test that goes over is made smaller or long, as below, never allowed more.

**A long test** is one that takes more than about a second in the test profile.
`#[cfg_attr(debug_assertions, ignore = "long: the quick check is quick_one")]` skips it in a non-release run, where
debug assertions are on, and `--release` runs it.  `quick_one` is the quick test that checks the same property at a
small size: a prefix of the range that the long test walks (which then takes the rest of it), a sample of the chain or
of the golden games, fewer random draws, a smaller game.  Where several tests walk one path at different sizes (a ledger
run for each of its features, a certify test for each refusal), the quick check keeps one small test of the path, and
the `ignore` message of each long one names the test that stands in for it.

**The budget.**  The target of 10 s on ten cores is a budget of about 80 CPU seconds for all the tests of the quick
check together, so a test that is added is paid for by shrinking another.  A few tests that draw a count, or build an
instance that must outlast a script, take a smaller one in a non-release run (`QUICK` in `tests/common/support.rs`,
shared with the unit tests: the engine's instances, the random configurations of `robustness`).  Tests with a plain
`#[ignore]` are ignored in both runs.

The *full suite* is `cargo test --release`: every test, the long ones too, about 3 minutes of tests after a build of 6
minutes.  It runs before a cloud run, after any change that changes search trees or results, and in the gates (the
one-thread re-solve of 1301-2175).  A default build, and so every run above and the cloud setup, covers the library, the
binary, the tests, and the examples `ledger`, `perf` and `ladder`.  The study tools (the examples `tune`, `extended`,
`structure` and `wall`, with the tests of `tune` and `wall`) are behind the Cargo feature `studies`: build them with
`--features studies`, and run `cargo test --features studies` and `cargo clippy --all-targets --features studies` after
a change to the library that they read.

**Before a change is merged** (what a reviewer of a subagent's work runs, besides reading it; the tests are described
below, the probe in section 10):

* every change: `solver/tools/quick_check.py`, and `cargo test --release`, `cargo clippy --all-targets` and `cargo fmt
  --check` in `solver/noframe/`;
* a test added or made bigger: `solver/tools/quick_check.py` again; it fails a test that is too slow, which is then
  shrunk, or made long with a `quick_one` (above);
* a change that can change a search tree or a result (the next three items): the full suite in `solver/frame/` besides;
* a bound, or anything that prunes: the bound-soundness tests and the soundness harness pass, and a bound never falls
  below the best completion of its node;
* a pure speedup: every search tree identical, so the probe's fingerprints and the node-count tripwires do not move;
* anything that touches the search, the chain or the results: `taxman regress --to 1000`; a scratch chain (`taxman
  solve 1-N --out FILE`, not in `results/`) compared with `results/strict.jsonl`; and both verifiers on it, since a
  results line carries provenance that they check (`solver/tools/cloud/verify.sh FILE` runs `taxman verify-results` and
  `solver/tools/check_results.py`, and compares with the committed chain);
* a change to an on-disk format (results line, checkpoint header, `testdata/`): the commit message says so, and why.

### The tests

What the full suite checks, by subject.  A few tests sample what an exhaustive run would cover (every 18th golden game
for the peeling agreement, every sixth game above 511 for the root promotability of the frame builder's candidates,
every second line of the Java fixtures), and the comment on each says what the whole run costs.

* **Unit tests**, per module, include the Java tests' cases (`FactorTableTest`, `FrameTest`, `FrameSetTest` for
  n = 85).  The helpers that the unit and integration tests share (temporary directories, a random generator) are one
  file, `tests/common/support.rs`.
* **Golden tests** (`tests/golden_frames.rs`, `tests/solver.rs`) check every stage of the port against the Java's
  answers in `testdata/golden/n*.json` ([its README](../frame/testdata/golden/README.md) says where they came from):
  the setup's frames, the numbered and merged frames, the candidates, the reduced frame set, the greedy incumbent, the
  prime shortcut and the result of the solve.  The searched golden games (94, up to 760) give the Java's promotion sums
  with the default tactics, and the tactic variants give the same answers on those up to 400.  Past 760 the Java's
  structure is fixed in `testdata/golden_wide/` (n = 511, 512, 1000, 1001, 1023, 1024, 1025, 1100, 1231, 2047 and 2048,
  checked at the width `with_width!` picks) and `games_java.txt` (the Java's game of the board's frames, move for move,
  for 171 sets of promotions at 762–2049).
* **The board layer against the chain** (`tests/board_layer.rs`): for every 25th searched game of
  `results/strict.jsonl`, and the last, the moves that the game's promotions make and the digest of the board's search
  problem are the line's, which the golden data (up to 2049) cannot say of the games that matter.  The unit tests of
  `frames` check what the frames remember (which numbers each can take on, which sources `removeImpossible` has been
  through) against a fresh computation.
* **The frame model's exactness** (THEORY.md, Theorems B and M; section 2, "What optimal means"):
  * `tests/model_exact.rs` decides playability on the whole board with an oracle that shares no code with `frames` or
    `model` (`src/playable.rs`, the peel, itself checked against the rules on every subset of the games to 20).  It
    checks both directions of Theorem M on every set of every game to 100 (to 120 ignored) and on random sets of games
    to 1300 (to 8191 ignored), and the near miss that only the candidate filter keeps out (n = 98).
  * `tests/model_extended.rs` checks Theorem M⁺ (the extended problem, with the sources of three or more prime factors
    optional) both ways, in both formulations (with and without the claims of `unzip`, Lemma M5⁺): on every set of
    every game to 80 (to 60 with every such source open) and on random sets to 1300 (to 4095 ignored).  It checks that
    with none open the promotion sets are the frame model's, and catches a mutant that skips the check of Lemma M5⁺.
  * `tests/bottom_layers.rs` checks Theorem B on every game of `optimal.json`, the committed chain and the chain past
    it, every optimal set the prover enumerated, and every optimal set of the games to 48.
  * The unit tests of `frames` check that `unzip`'s claims are its mandatory pairs, that `numbering` is what
    `build_frames` uses, that the frames lie in layers (Lemma Ω, also on punctured boards), and that the numbering by
    layer is the Java's breadth-first numbering on every game to 400 and a sample to 8191.
  * The census of every game (`examples/structure.rs`; its data are in
    `solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`) checks what Theorem M's proof says of setup
    one game at a time, and that Π⁺(∅) with the claims of `unzip` is Π_n (check H).
* **Regression**: `taxman regress --to N` solves 1..N as one range and checks every score against `optimal.json`.
* **Tactic invariance**: games 1..300, forced to search, give identical scores with each tactic off or changed, and a
  checkpoint is refused when any of the search tactics differs.
* **Bound soundness**:
  * on small frame sets, the upper bound is never below the best completion computed exhaustively; the same for the
    matching bound and its refinement by conflicts (whose exact completions must be promotable, and whose conflicts,
    carried down a path, must stay conflicts), and for the Lagrangian bound over the root's nogoods with fitted and
    random multipliers;
  * the incremental solvers agree with the reference min-cost flow (with the Lagrangian weights, live nogoods and
    constant worked out afresh, under random relaxations changed mid-walk) and with a solver started afresh, and their
    levels pass the optimality certificate;
  * the GF(2) bound (`src/gf2/tests.rs`, `src/gf2/meet.rs`): the intersection against brute force under every size
    limit, from every set it reports as a warm start, with members deleted, and its weight split's bounds without each
    member.  On the games up to 400 the bound is never below the exhaustive best completion, and the optimal
    promotions are independent in both matroids.  Along random paths of games 60–1000 (promotions, deletions,
    backtracking) every level equals the matroids built afresh at its node, every member it calls forced is (the bound
    without it, solved afresh, is below what the node needs), and every tableau derived instead of built is the one a
    solve builds; along the same kind of paths the copies of every word count (1 to 5, padded) give the same results
    and do the same work.  Blocks of more than 255 members (five words) are checked against the greedy algorithm, one
    matroid free and the other random.  At the 600 nodes of the probe's fixtures (936 and 1215), completions promoted
    in random orders are independent in the node's matroids and no heavier than its bound, and the matching path's
    search started from the bound finds nothing above it (`tests/soundness.rs`);
  * conflicts and residuals are checked against promoting the sets;
  * the prime exchanges (`src/search/symmetry.rs`): the symmetric pairs against their definition (every number looked
    at, every pair of primes tried) on the games 2..=300 and a sample to 2200; the images of the optimal set and of
    random maximal promotable sets promotable and worth the formula; every optimal set keeping every inequality; on the
    games with at most 22 candidates, every promotable set so checked, and the best that keeps the inequalities worth
    P\*; and the rule's cuts and drops against its definition by enumeration (exact where the pairs are independent).
* **Soundness harness** (`tests/soundness.rs`): the search starts from an incumbent worth 1 or 5 less than P\* with an
  empty payload, so that any pruning that loses every best completion leaves the fake incumbent in place (which tests
  the `<` and `<=` boundaries too).  It runs:
  * every game 2..=300 under each of 34 configurations: 19 of the matching path (every tactic; forgetting every step,
    every third, never; the Lagrangian bound refitted every few steps; one thread and two), 11 of the default, the
    GF(2) bound with the rule of symmetric pairs (every way of choosing the branch; the matching bound as a first check;
    without forced members or nogoods; forgetting every step; two threads; the Lagrangian bound after it), the GF(2)
    bound without the rule on one thread, two, and forgetting every step, and the matching path with the rule;
  * every game 2..=600 with symmetric pairs, with the rule over either bound (and the oracle without the frame model to
    40); units on the way to an optimal set (an optimal set less 30 to 70% of its members, games 600..1000) with the
    rule; and games 301..=600 (every tenth: searches of hundreds to thousands of steps) with the Lagrangian bound
    refitted early and often, over either bound, where it cuts and forces thousands of times;
  * the same for 500 perturbed frame sets (numbers dropped, a role flipped, moved a frame) with the truth found
    exhaustively; promoting a set at once against one by one in random orders; conflicts and nogoods on perturbed
    models;
  * searches interrupted every 1,000 nodes and resumed from a checkpoint with units spilling: game 600 on the matching
    path, one of them with the Lagrangian bound refitted every four steps, and game 1000 with the GF(2) bound, whose
    search of 600 is too short to stop;
  * units of real games at W = 20 to 64 (thinned so that the truth is found exhaustively) from a handicapped incumbent,
    and games 1133 (matching) and 1000 (GF(2)) resumed every 1,024 steps at W = 20, 24 and 64, which carry the harness
    past the widths of the Java's table (n ≤ 1000).

  A search below the root with the rule of symmetric pairs is held to the best completion that keeps their
  inequalities (an exhaustive oracle of its own, which prunes by the same bound but shares no code with the rule), which
  it must reach; at the root that is P\* (Theorem 13).
* **The floor** (`src/search/tests.rs`): on every game of the Java's that needs a search, to 400, a floor below P\*
  finds an optimal set, a floor at P\* or above completes with nothing found, and a floor at the root's bound or above
  ends at the first check; with the default tactics and (every ninth game) on four threads, on the matching path, and
  without the rule of symmetric pairs.  The floor at P\* searches the tree of the search seeded with an optimal set,
  step for step, on games 300 to 1000; a floor at the ceiling ends the search before it starts; a stopped floor search
  resumes under its floor, and is refused under another floor or none.  With no floor nothing moves: the tripwires and
  the probe's fingerprints are the same.
* **Node-count tripwires** (`tests/tripwires.rs`): a pruning tactic that stops working changes no answer, so no
  assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` takes 27 to 67% more nodes).
  With one thread the tree is deterministic, so the test asserts the exact node count of a fixed game under fixed
  tactics: n = 480 to 1133, every tactic off or changed in turn, with the GF(2) bound and on the matching path, and the
  tree of game 600 on the matching path with the searchers forgetting their nogoods every 500 steps.  The matching
  path's rows are the default's trees before the GF(2) bound, its rows with `lagrange=off` those of the build before
  that tactic, and the rows `symmetry=off` the default's before the rule of symmetric pairs.  The table `EXTENDED` does
  the same for the extended problem with nothing open, Π⁺(∅), without the claims of `unzip`, whose search must also be
  the default engine's on the same problem, node for node; with the claims, Π⁺(∅) must pose the default problem itself
  and reproduce every row of the default table, node for node.

  **A change that alters the tree on purpose fails them.**  Run `cargo test --release --test tripwires -- --nocapture`
  (the quick tests check only the rows of trees of 400 nodes or fewer): it prints the rows as observed, in the layout of
  the table at the top of the file.  Check that the promotion sums (the answers) have not moved, paste the rows in, and
  update any node counts quoted in the docs.
* **Extended mode** (section 4, "Extended mode: one-sided candidates"):
  * the model with one-sided candidates against the extended problem's reference predicate on every game 2..=300, for
    nothing open, every source of three or more prime factors open (to 120) and five random sets of them
    (`tests/model_extended.rs`: sets promoted at once and one by one in random orders, their conflicts and nogoods, the
    narrowing and the count);
  * its search against the whole-board oracle on every game to 80 (to 60 with every such source open, 69 ignored);
  * the GF(2) bound's free blocks: the intersection against brute force with coloops, every promotion set of the games
    to 120 independent and under the bound, and random paths to 1300 matching the matroids built afresh at every node;
  * the soundness harness in extended mode on every game 2..=300 under eight configurations, and a game resumed every
    1,000 nodes;
  * the header's digest of the one-sided candidates, and the refusals of what extended mode cannot take (tactics, block
    words, another problem's checkpoint).
* **Certificates** (`tests/certify.rs`):
  * `taxman certify`: every game of 600..=1300 that needs a certificate certified (ignored; a sample by default), and
    some past 1300; a planted wrong score beaten by a game the referee accepts, with the status 2 and the
    counterexample file, and said again by a rerun; a certificate's search stopped every 1,000 nodes and resumed to the
    same verdict; the games a certificate cannot be made for refused; the file of certificates and the bounds' files
    read as they should be; `--list` certifying the games of a file in its order;
  * `taxman verify-certificates`: a file that `certify` wrote over 1312..=1317 (three games that rest on the game before
    them) passes, on one thread and on two, with the summary's counts and the committed prover files' proven end of the
    chain.  Each doctored line (another digest, score, open members or width; a `same_problem` with nothing to rest on,
    or with its numbers changed; a certified line with a counterexample; a `build` without a commit; a score the
    results do not have) is refused, naming the line.  A beaten line is verified with its counterexample, and cannot
    stand beside a certified one.  The end of the chain is checked on synthetic data (a gap stops what rests on it) and
    on the committed chain;
  * the two formulations: a line records its formulation, and an old line reads as without the claims.  The rule
    settles the same pairs in each formulation and across them (a certificate of one settles a later game in the
    other, with the digest of the problem in the run's formulation, and as a search of the game does).  The claims
    change the problems of 231 of the 377 games of 600..=1300 that need a certificate, and none of their sizes, and make
    the rule settle 101 games where it settled 81 (609 and 611, say, pose different problems without the claims and one
    with them).  A file that mixes the formulations passes, and a line whose formulation is changed is refused.
* **Engine** (`tests/engine.rs`, `tests/robustness.rs`, `tests/stop_signal.rs`):
  * a toy problem with a known answer (knapsack, against dynamic programming), run with 1 and many threads, with a tiny
    `queue_mem_cap` to force spilling, and killed and resumed from checkpoints, gives the same answer.  Ignored tests do
    the same with 64 and 192 threads, checkpoints every 2 ms, and a panicking worker (`cargo test --release --test
    engine -- --ignored many_threads`), and draw random configurations through any number of resumes
    (`TXENGINE_STRESS=<n>`);
  * a tree with no pruning checks conservation exactly: under random configurations the nodes counted must equal the
    tree's size, so one unit lost or explored twice shows (`TXENGINE_CONSERVATION=<n>` for more);
  * robustness: an interval of zero finishes; limits too large for the clock are never reached; a deadline that has
    passed, or a stop signal received, before a run starts stops it before any search, with every unit in its
    checkpoint; spill segments of dead processes are removed; a locked checkpoint directory or results file is
    refused; a checkpoint of another game, with other tactics, or with an incumbent that does not fit is refused; and
    the binary rejects bad times, survives a closed stdout and stderr, and stops on SIGINT and SIGTERM with a checkpoint
    it then resumes;
  * the control: a ceiling given before a run, or while it runs at the incumbent it has, ends it as optimal (with no
    search work if the start reaches it); offers from outside raise the incumbent; a stop through it checkpoints a run
    that resumes exactly; workers added, retired and down to none (suspended, then resumed, checkpointed, stopped, or
    ended by a ceiling) give the exact answer; and the conservation test runs with a controller that sets 0 to 4
    workers every millisecond or so while the run checkpoints, spills and resumes;
  * the counters: capacity is wall × threads when the workers do not change, and a searcher that sleeps before its
    first and second polls shows in the rebuild and first-step times.
* **The window** (`tests/window.rs`; the policy's unit tests in `src/solver/window/allot.rs`):
  * chains 1..600 with windows of 1, 2, 4 and 8 on 4 threads have the scores, methods, proofs, ceilings, assumptions,
    repeats and previous games of the committed chain, and pass `verify_files`; with one thread any window gives the
    lines of a window of one, but for the times and the build;
  * a repeat of a game still in flight (301 of 300) names it, on a later line;
  * a range stopped by a node limit ends at its first unfinished game, with the lines before it in order, and completes
    when run again;
  * a game that passes the wall (`game_limit`) ends the range with no line for it, its checkpoint and clock kept, walls
    again at once under the same limit from the clock it left, and completes under none, leaving no clock (the command
    through the binary is in `tests/cli.rs`, the clock file's unit tests in `src/solver/clock.rs`);
  * a score that is not the table's fails the range at that game, with the lines before it, as a window of one does;
  * the policy: claims served oldest first, a lone ready search takes every free thread, a busy one doubles, idle
    workers go back only to another claim and after the last grant has settled, the oldest search reclaims from the
    youngest first and suspends them if need be, and a suspended search claims one thread.
* **The cloud scripts** have tests of their own that need no cloud: the scripts that run the chain unattended
  (`solver/tools/cloud/test/run.sh`: the wall and the ledger through `chain.sh`, `wall-node.sh` through each way a run
  ends, `wall.sh` through a launch, a stop and a clean-up, against a stand-in for the aws CLI; [RUNBOOK, section
  5.6](RUNBOOK.md#56-what-was-tested-and-what-can-go-wrong)), and the build cache and the release suite that runs on
  EC2 before binaries are stored (`solver/tools/cloud/test/test_build_cache.py`, `test_run_tests.py` and
  `test_setup.py`, run with `python3 -I`; [RUNBOOK, section 11](RUNBOOK.md#11-the-build-cache-and-the-release-suite)).
* **Integrity of results** (`tests/integrity.rs`, `tests/results.rs`, `tests/solver.rs`): a previous game that is not
  what it says is refused; an assumption travels down a chain; a score that is not the table's is a hard error; a
  tampered results file, or one whose line contradicts itself or is of another schema, is refused by both verifiers,
  naming the line; a torn last line is dropped; the committed results pass the verifier with the counts that
  `results/README.md` quotes; a game stopped by a node limit and resumed reads back assuming nothing.  Both verifiers
  refuse forged repeats (another game named; the digests, promotions or promotion sum changed; the game repeated
  unfinished, after the repeat, or settling no search; its assumptions dropped), and `verify-results` refuses two more
  that only setting the problems up again can catch (the Python referee accepts them, as its docstring says).
* **Keeping going past limits** (`tests/keep_going.rs`, `tests/results.rs`, `tests/sweep.rs`,
  `solver/tools/test_sweep_plan.py`, `solver/tools/cloud/test/run.sh sweep`):
  * a range whose games all stop at a limit goes on, each game built on the best game of the one before it, with no
    ceiling; a prime game after an unfinished one is searched, and after a proven one derived;
  * a rerun passes over the unfinished games; `--retry` and `--retry-improved` search them again (the latter only where
    the game before has gained, the rule also in a unit test of `window`), and the lines name the pass;
  * both verifiers accept a file with holes and unfinished lines, and refuse a ceiling, a `cap` or `prime_formula`
    proof, a changed score, moves or assumptions, or a later line, for a game built on one;
  * the sweep's planner tiles a range into blocks of equal work and cuts the excerpts of the passes, and `sweep.sh` over
    1301..1340 and 1411..1416 completes with both referees accepting the file, is stopped by a SIGTERM and resumed, and
    settles a block's first game by the ceiling its neighbour gives in pass 2.
* **Repeats** (`tests/same_search.rs`): the games that repeat an earlier search are found from the setups alone, and a
  chain concludes exactly those, with the scores of a chain that searches them, from n = 1 (1..300) and from a results
  file that a stopped run left; a game with a checkpoint is resumed, not repeated; a repeat assumes what the game it
  repeats assumes.  A unit test of `board` checks that the problem set up again from n is the one the setup posed, and
  the same at two set widths.
* **Known games** (`tests/known.rs`, with hand-made sources: a store of games 906..944 proven with the truth's games, a
  committed chain of games 1..60, and the table):
  * a chain over 900..960 records the store's proven games with no search, citing their lines and assuming U at the
    games they rest on, and the others with the scores of a chain without the store (and fewer nodes in all); a game
    carried from the game of another line is played and recorded;
  * the committed chain's and the table's games are recorded with their own assumptions, and the game before the range
    from each of the three has the source in `prev`;
  * an open game's upper bound ends its search (proof `known`, citing the one line), and its lower bound's game seeds
    it (fewer nodes); `force_search` leaves the games to their searches; a store game that does not play, or that the
    chain bounds contradict, stops the run at its game;
  * both verifiers accept those files, and refuse each of: an upper bound of the store worth another value, a missing
    lower or upper line, a line by another rule or from another run, no store given, a citation of other lines,
    assumptions dropped or added, no citation, a changed proof or score, a citation on a game that is not a known one.
    `tests/docs.rs` has `known` among the keys a line has.
* **Large games** (`tests/wide.rs`): the Java stopped at 1000, and the sets that hold a game are 8 to 128 words wide,
  so nothing above 1023 meets anything but the committed results and these tests.
  * The same game at every width that holds it (128 included) has the same frames, candidates, greedy set,
    `max_total`, answer and tree.
  * Game 1133 (W = 20, about 2,400 nodes) under every tactic against `strict.jsonl`, and a slice of that chain; the
    sacrifice bound proves game 1133 by an independent route.
  * A game of 2050 built at W = 64 (it dispatches to 36) and one of 4100 at 128 go through the frames, the matching
    bound, conflicts, the greedy set and a node-limited search; games of 2050, 2400, 2800 and 3100 at the widths they
    dispatch to (36, 40, 48 and 56) and at 64 (2050 also at 128) have the same start and the same node-limited search,
    and 4095 at 64 and 128 the same start.
  * The games past 4095 (4096, 4100, 4740, 6000, 6392, 8191) at 128 words are checked against the board layer's own
    invariants: nine frames, the plain game, the freedoms, the greedy set and subsets of it promoted and played to the
    score their sum says, and the root's GF(2) bound between the greedy sum and the matching bound, with three, four and
    five block words.  The GF(2) bound's copies of four and five words (only at 128) search game 1000's tree as the
    copy of one word does.
  * A committed checkpoint of game 1133 (`testdata/n1133_after_1000_nodes.ckpt`, W = 20) must still resume: a
    deliberate change of the checkpoint format fails it, and the test says how to make the file again.
  * The levels of a real depth-first stack of game 1215 (`testdata/n1215_deep_units.txt`): the incremental matching
    bound and the quick bounds without a member against the min-cost-flow reference; and promoting one number at a time
    (with its no-peel shortcut) against the peel of every frame, in reached states of games at W = 20, 28 and 64.  The
    unit tests check the nogood table's lookups against their definition at compact strides of 1 to 16 words, and debug
    builds check that every root frame is winnable, which the shortcut needs.
* **Command line** (`tests/cli.rs`): every subcommand's happy path and its usage errors, through the real binary;
  `bench --history FILE` keeps the tracked `solver/frame/bench/history.jsonl` out of the tests.
* **Documentation** (`tests/docs.rs`): what the docs state that the code can check, from the table of tactics to
  the links, the citations of headings, the sizes of [WRITING.md](../../docs/WRITING.md)'s rule 8 and the words the
  project has retired.  The comment at the top of the file lists every check.

## 10. Performance

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other
load on the machine moves wall times by up to 3×.  Five tools measure it:

* **`taxman bench`** runs a fixed set of games that need a search and take about a second each on one thread: n = 936
  (W = 16), and 1050, 1100, 1133, 1170 and 1184 (W = 20).  It solves them standalone, with acceleration off (the game
  of n − 1 from `optimal.json`, past it from `results/strict.jsonl`), prints the times of each phase and the node
  counts, and appends them to `solver/frame/bench/history.jsonl`, so that performance can be followed across changes.
  Each line has the commit the binary was built from, every tactic's value, and for each game the nodes, the seconds
  of each phase and the search's counters.  `--history FILE` and `--known FILE` name other files; by default the
  command finds them in the repository.
* **The probe** (`cargo run --release --example perf`) replays the hot paths of the hardest games from fixtures in
  `testdata/perf/` (0.2 MB), and measures them in cycles and instructions retired of the process (`proc_pid_rusage`).
  Each part has a fingerprint of what it computes, which an exact optimization must leave unchanged:
  * F1: 300 search nodes from each of the proofs of 936 and 1215, every kernel over all of them (the rebuild of a node,
    the promotion of every candidate, conflicts, the matching from scratch and warm-started, the bounds without each
    member, strong branching's exact solves, children, nogood lookups and rule-outs);
  * F2: the frame checks of those promotions;
  * F3: a checkpoint of each proof (`perf make-window 936 4500`, `perf make-window 1215 600000`), resumed on one thread
    for 50,000 steps, one period of forgetting nogoods: the search itself, fingerprinted by its counters, with the
    tactics its checkpoint records, the matching path's (`--tactic` changes them; a search that ends inside the window
    is costed by the steps it took);
  * F4, with `--tactic gf2=on`: the GF(2) bound's kernels at the F1 nodes (the bound from scratch; with its weight
    split and forced members; without each member of its heaviest set's conflict, solved, and as strong branching
    reads them, certified or solved), and the windows resumed with the GF(2) bound, which finish both searches.  A
    window's fingerprint leaves out the GF(2) bound's counts of solves and augmentations, which measure work.  The
    kernels and the windows search the forced members as `--tactic force_window` says (the default, `all`).

  The probe's fingerprint is 7e8e724717d26545, 5b83bbebf0bb2899 with `--tactic lagrange=off`, a58ade4629ab6186 with
  `--tactic gf2=on`, and f3deaac0dbdc030a with `--tactic gf2=on --tactic force_window=80`.  It takes about 5 s in all;
  `perf make-nodes` and `perf make-window` make the fixtures again, deterministically.  For whole searches,
  `/usr/bin/time -l` reports the same counters for a process.  `cargo test --release` rebuilds the examples in its
  own configuration, with about 2% more instructions: run `cargo build --release --examples` again before measuring.
* **Profile-guided optimization** (`solver/tools/pgo.sh`) gains 0 to 2%, with the same trees and fingerprints, since
  the hot loop is inlined by hand.  An instrumented build runs the probe and a short search at every set width (code
  that a profile never saw is optimized as cold), and `-Cprofile-use` builds `target/pgo/release/taxman` and the
  examples.  Profile the plain build: with PGO the search is inlined into a few functions, and a profiler sees little.
  On Linux, rustup's `llvm-tools` has an llvm-profdata that matches rustc's LLVM, and the script adds whatever
  `RUSTFLAGS` it is given (for instance `-C target-cpu=native`, which a plain build does not use) to the profile flags.
* **The ladder** (`cargo run --release --example ladder -- [--threads T[,T…] | --scaling] [--seconds S] [--diag]
  [--seed-opt] [--known FILE] [--tactic …] [--node-limit N] N…`; the top of `examples/ladder.rs` documents its flags)
  makes standalone searches of hard games from the frame builder, seeded from the known game of n − 1 and capped as
  the solver does, each with a time limit.  It reports completion, nodes, time, when the final incumbent was found,
  and the engine's counters.  `--seed-opt` seeds with an optimal game of n itself, to measure the proof alone; `--diag`
  prints `search::Diagnostics` (nodes by depth, what pruned, bound gaps, conflict sizes, matching work).  Games for
  fast progress benchmarks (one thread): 880, 936, 1050, 1100, 1170, 1184; harder ones: 992, 996, 1025, 1215, 1216 on
  several threads.
* **Scaling** is the ladder with a list of thread counts (`--threads 1,2,4,8`, or `--scaling` for 1, 2, 4, … up to all
  the CPUs).  It searches each game with each count, and prints a table for each game and one summed over the games:
  search time, speedup, nodes per second per thread, effective parallelism, idle and lock-wait time, splits and the
  units they gave, steals, runs, tiny runs, and the time to full occupancy.  With `--seed-opt` the node count barely
  depends on the threads, so the speedup measures the parallel proof alone.  `--cores C` lets only C of the threads
  search at once (`EngineConfig::cores`), to measure in counts, not time, what many more workers than CPUs would do.
  The engine's own benchmark (`cargo test --release --test engine -- --ignored --nocapture bench`, its sizes and thread
  counts set by `TXENGINE_BENCH_*`) measures the engine's overhead per node, with no work per node.

The cloud scripts (`solver/tools/cloud`) run the scaling sweep, the cost of a checkpoint pause and the chain on a
machine of many CPUs ([RUNBOOK.md, sections 3 and 4](RUNBOOK.md#3-before-a-big-run)).

**Where it stands.**  At the end of the chain a step of the search cost 888k cycles on one thread of the M1 Max, 92% of
it in the GF(2) bound, and [cheaper GF(2) nodes](../notebook/cheaper-gf2-nodes-same-trees-2026-10/README.md) took it to
543k, with the same trees.  The cost of a game comes in waves that follow the root's GF(2) gap, and past 2175 it grows
tenfold for about every 1,040 in n ([the chain to 3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md));
[results/README.md](../../results/README.md#how-the-file-was-made) says how long the committed chain took.  The
measurements of the earlier builds, from the matching path to the GF(2) bound's rounds, are in [the benchmarks
entry](../notebook/benchmarks-the-probe-pgo-and-scaling/README.md#later-2026-10-the-measurements-that-designmd-held) and
the entries of each round.
