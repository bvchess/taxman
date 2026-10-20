# Architecture

Part of the [design](../DESIGN.md).

The frame solver is one Cargo crate, `solver/frame/`: a library, the binary `taxman`, examples and integration tests,
with no third-party dependencies.  The library's modules form layers, each using only the layers before it, and a solve
passes through all of them but the studies.  The module map of [`src/lib.rs`](../../frame/src/lib.rs) lists the modules
in that order, a line each.  The list below follows the same order, with advice for a first read.  Each module starts
with a comment that says what it is for; read it first.  The pipeline of [Per-game pipeline
(`solver`)](engine.md#per-game-pipeline-solver) shows how the pieces fit together, so keep it open.

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
  between games; [The ledger](trust.md#the-ledger));
* behind the Cargo feature `studies` (`--features studies`), the study tools: `tune` (the tuning benchmark: units of a
  search at the end of the chain, each solved on one thread), `structure` (the census of the frames' structure, game by
  game), `extended` (chains and problems of the extended problem) and `wall` (what grows with n and with a tree: a
  game's fixed costs piece by piece, a node's cost at any set width, large trees with their memory and checkpoints, the
  frames of any game, and the `landscape` of a range of games, each solved standalone: the root's GF(2) bound over the
  incumbent a search starts from).

`tests/` holds the integration tests ([testing.md](testing.md)), and `solver/tools/` the scripts around the programs:
the second verifiers in Python, the quick check, the profile-guided build and the cloud scripts ([solver/README.md, "The
tools"](../../README.md#the-tools)).  The programs of finished studies (`fields`, `matroid`, `dominance`, `slope`,
`repeats`, `promotions`, `diff`, `hunt` and `sacrifice`) are gone from master; [the notebook's list of retired
terms](../../notebook/terms-changed.md#removed-programs) says where they build.

**Width dispatch.**  A set of game numbers is a bitset of W words of 64 bits, and everything that holds one is generic
over `const W: usize`.  The macro `with_width!` (in `bits`; the solver, the examples and the tests use it) picks the
narrowest width of `bits::WIDTHS` that holds n:

| W | 8 | 16 | 20 | 24 | 28 | 32 | 36 | 40 | 48 | 56 | 64 | 128 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| n up to | 511 | 1023 | 1279 | 1535 | 1791 | 2047 | 2303 | 2559 | 3071 | 3583 | 4095 | 8191 |

Every set operation loops over all W words, so a narrower set makes every step cheaper, and the steps of 4 to 8 words
between 1023 and 4095 pay.  Past 4095 they would not: a node at 4100 costs 1% more at 128 words than at 72, with the
board layer the same, so 128 is the only width there ([boards past
4095](../../notebook/boards-past-4095-2026-10/README.md)).  The price is build time, since each width is a copy of the
search's code: a release build takes about 100 seconds on an M1 Max, about 12 more for each width ([the
engine](../../notebook/the-engine/README.md) has the times as widths were added).  The largest game, 8191
(`solver::MAX_N`, from the widest width), also sets the other fixed sizes, each checked on every game to 8191: at most
nine frames (`model::MAX_FRAMES`), a peel's worklist of two summary words, and GF(2) vectors of at most five words ([The
GF(2) bound](gf2-bound.md#the-gf2-bound)).

*Details.*  A checkpoint records its width, and a build refuses a checkpoint made at another width than it gives the
game.  So a checkpoint of a game of 2048 to 3583 made at 64 words, by a build before the widths of 36, 40, 48 and 56
words, does not resume on a build that has them.  Adding 128 moved no game, so the checkpoints of the games to 4095
resume on the builds past it.
