# The frame solver

The frame solver finds optimal games of Taxman.  It splits each game into largely independent groups of numbers, the
[frames](../../docs/GLOSSARY.md#the-frame-model), and searches for the best set of numbers to play beyond the ones the
frames settle.  It can run on many threads, and its long runs can be resumed after an interruption.  Below, "the solver"
is the frame solver.

This page is its guide: what it computes, how to build and run it, and the common tasks.  [The
reference](../docs/REFERENCE.md) has every flag and command, how to read the output, and why the answers can be
trusted.  [DESIGN.md](../docs/DESIGN.md) explains how the solver works and why, the [lab
notebook](../notebook/README.md) holds the measurements behind it, and the [glossary](../../docs/GLOSSARY.md) defines
the project's words.  A second program, [the prover](../noframe/README.md), checks the solver's answers on the whole
board, without the frame model.

* [1. What it solves](#1-what-it-solves)
* [2. Build, test, run](#2-build-test-run)
* [3. Commands](#3-commands)

## 1. What it solves

### The game

The pot holds the numbers 1 to n.  The player takes a number c that still has a proper divisor in the pot, and scores
c.  The taxman then takes every divisor of c still in the pot.  The game ends when no number in the pot has a proper
divisor left in it, and the taxman takes whatever remains.  An *optimal game* scores the most, opt(n).  Game 4 has the
optimal game 3, 4: the taxman takes 1, then 2, and the player scores 7.  The values of opt(n) are the Taxman sequence
([OEIS A019312](https://oeis.org/A019312)).

The solver's results are optimal games, each a sequence of moves with its score.  The file `results/strict.jsonl` holds
a chain of them for every n from 1 to its end, each built from the one before (section 3, "A chain from n = 1"); this
page calls it the *committed chain*.  The code can attempt any n up to 8191, the largest board that its bitsets of game
numbers can hold ([solver/docs/design/architecture.md](../docs/design/architecture.md), the paragraph "Width
dispatch").

[results/README.md](../../results/README.md) gives how far the chain reaches and how each game is proven: within the
frame model, given statement U (below, "What optimal means"), and up to a smaller n also on the whole board, by the
prover.

*What a game costs.*  Finding an optimal game can turn into a search over an enormous tree, and its cost comes in waves
of cheap and dear games.  How fast it grows with n depends on the range.  The first 2175 games of the committed chain
took 27 minutes on eight threads of a laptop; the chain goes on past them ([how far it
reaches](../../results/README.md#the-files)).  Over the games 1301 to 2175 the waves show no growth that a fit resolves
(the notebook's study of [the GF(2) bound](../notebook/search/the-gf2-bound-in-the-search-2026-10/README.md), which lets
the search cut branches).  Past 2175 a fit does find growth: the search times of the games 2176 to 3440 rise tenfold for
about every 1,040 in n, with about a decade of scatter ([the chain to
3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md)).

### How the solver sees the game (the frame model)

The solver does not search the game move by move.  It splits the numbers into largely independent groups, settles what
it can before it searches, and then searches only over which spare numbers to play as moves.  This decomposition is the
[frame model](../../docs/GLOSSARY.md#the-frame-model).  The steps below follow the solver's setup, and each term links
to its glossary entry.

1. **Sources and factors.**  The numbers above n/2 are [sources](../../docs/GLOSSARY.md#the-frame-model): each divides
   nothing else on the board, so it is a move or nothing.  Every other number is a
   [factor](../../docs/GLOSSARY.md#the-frame-model), which serves as tax and sometimes as a move.
2. **Maximal factors are enough.**  A move needs only one of its [maximal factors](../../docs/GLOSSARY.md#the-game) to
   be still in the pot, and that factor pays for it, whatever else the taxman takes with the move.  This is the [lifting
   argument](../../docs/GLOSSARY.md#the-game) of [src/numtheory.rs](src/numtheory.rs), which
   [THEORY.md](../../docs/THEORY.md) proves as Theorem 1.
3. **Frames.**  Sources that share a factor form a [frame](../../docs/GLOSSARY.md#the-frame-model), with the factors
   that can pay for them.  Frame X is [in front of](../../docs/GLOSSARY.md#the-frame-model) frame Y, and Y
   [behind](../../docs/GLOSSARY.md#the-frame-model) X, when a number of Y has a maximal factor in X; the frames in front
   are played first.  Each frame plays a small game, [Taxman Mini](../../docs/GLOSSARY.md#the-frame-model), and is
   [winnable](../../docs/GLOSSARY.md#the-frame-model) when some order takes all its moves.
4. **Setup.**  Before the search, setup simplifies each frame.  It drops the [impossible
   sources](../../docs/GLOSSARY.md#the-frame-model), settles the [mandatory
   moves](../../docs/GLOSSARY.md#the-frame-model), and counts the frame's
   [freedoms](../../docs/GLOSSARY.md#the-frame-model): how many factors it can spare.  A frame with none is
   [closed](../../docs/GLOSSARY.md#the-frame-model) and takes no part in the search.
5. **Numbering.**  Setup numbers the frames that have freedoms from the front, by
   [layer](../../docs/GLOSSARY.md#the-frame-model), the number of prime factors of their sources: frame 0 is the frames
   with the fewest, frame 1 those with one more, and so on.  Frames with the same [frame
   number](../../docs/GLOSSARY.md#the-frame-model) are merged into one, so that a promotion always has one frame to go
   to.
6. **Promotions.**  What a game can still choose is which spare factors to
   [promote](../../docs/GLOSSARY.md#the-frame-model): to play as moves instead of leaving them as tax.  A promotion
   makes a factor of frame h a move of frame h − 1, one frame forward, and both frames must stay winnable.  Nothing
   further forward is needed, which [docs/theory/frame-model.md](../../docs/theory/frame-model.md)
   proves.
7. **The score.**  A game of the model scores its [`in_the_bag`](../../docs/GLOSSARY.md#the-frame-model) plus the sum of
   its [promotion set](../../docs/GLOSSARY.md#the-frame-model).  So the solver searches for the promotion set with the
   largest sum, [P\*](../../docs/GLOSSARY.md#the-frame-model), by a branch and bound over which
   [candidates](../../docs/GLOSSARY.md#the-frame-model) to promote.  The moves of the game then include the promotions
   (`frames::BoardFrames`; [solver/docs/design/engine.md, "Per-game pipeline
   (`solver`)"](../docs/design/engine.md#per-game-pipeline-solver)).

**An example: game 21.**  The board 1 to 21 has four frames.  Two of them are closed, so they have no number.
`testdata/golden/n0021.json` has the frames, and the wiki's
[walkthrough of game 21](https://github.com/bvchess/taxman/wiki/Walkthrough-for-N=21) shows them in pictures:

```
closed    sources 19 11 13 17   factors 1           11, 13 and 17 are impossible (the 1 pays for 19)
frame 0   sources 14 15 21      factors 2 3 5 7     1 freedom (15 must take 5)
frame 1   sources 12 18 20      factors 4 6 9 10    1 freedom (20 must take 10)
closed    sources 16            factors 8
candidates: 4 9 (factors of frame 1 that frame 0 can accept as moves and still play all its own)
```

Frame 0 is in front of frame 1, because 9, a factor of frame 1, has the maximal factor 3 in frame 0.  The game with no
promotions plays every source that is not impossible, and scores `in_the_bag` = 135 (19 + 14 + 15 + 21 + 12 + 18 +
20 + 16).  Promoting 9 takes it out of frame 1's factors and makes it a move of frame 0, and the game scores 144;
promoting 4 instead would score 139.  Both frames stay winnable.

After a promotion the frames are [reduced](../../docs/GLOSSARY.md#the-frame-model): a move left with one live factor is
paid by it, so the pair drops out, repeatedly.  Here 9's only maximal factor is 3, so the pair (9, 3) drops out of
frame 0.  That leaves 21 only the factor 7 and 14 only the factor 2, so they drop out too.  In frame 1, 18 is left with
6 and 12 with 4.  Frames 0 and 1 have each spent their one freedom, and the game plays the frames from the front:
19; 9, 15, 21, 14; 18, 12, 20; 16.

The search is over such choices, and a larger game has more of them: the glossary counts the
[candidates](../../docs/GLOSSARY.md#the-frame-model) of three games.

Frames are first merged at game 20.  There the frames of 14 and of 15 share no factor, but neither has a frame with
freedoms in front of it, so both are frame 0.  Both sit directly in front of the frame of 12, 18 and 20, whose factor 6
has a maximal factor in each (2 and 3).  A promotion from that frame needs one frame to go to, so the two are merged
into one frame 0 with two freedoms.

### What "optimal" means

The solver's answer for game n is `in_the_bag` + [P\*](../../docs/GLOSSARY.md#the-frame-model).  That is opt(n) given
three things: the frame model is exact at n, which it is exactly when [statement
U](../../docs/GLOSSARY.md#the-frame-model) holds at n (a proof by the ceiling or the prime formula uses no model); the
search prunes nothing that could win, since every [tactic](../../docs/GLOSSARY.md#the-search) is proven or sound; and,
for some proofs, the game of n − 1 is optimal, so a [chain](../../docs/GLOSSARY.md#results-and-trust) of games from n =
1 is proven game by game.  So a result rests on nothing but U, where its proof uses the frame model; the correctness of
the code; and, where its proof says so, the game it was built from.  U is open in general.  It holds at every game the
[prover](../noframe/README.md) has proven, and `taxman certify` checks it game by game past them.
[solver/docs/design/frame-model.md, “What "optimal" means”](../docs/design/frame-model.md#what-optimal-means) explains
each premise and what each kind of proof needs of the model, [the
reference](../docs/REFERENCE.md#4-why-the-answers-can-be-trusted) says how each result records what it rests on, and
[results/README.md](../../results/README.md) counts the committed games of each kind of proof.

## 2. Build, test, run

### Build

`solver/frame/` is a Cargo crate: a library and one binary, `taxman`.  It runs on Linux or macOS: the engine uses POSIX
signals and `flock`.  It needs only a C linker and a Rust toolchain of at least 1.89 (`rust-version` in `Cargo.toml`;
the engine uses `File::try_lock`), and it uses no third-party libraries.  The independent verifier,
`solver/tools/check_results.py`, needs only Python 3 and its standard library.

```
cd solver/frame
cargo build --release              # about 100 seconds on an M1 Max; the binary is target/release/taxman
cargo build --release --examples   # also the ladder (scaling and progress harness), the probe and the ledger
cargo build --release --examples --features studies   # and the study tools: the tuning benchmark, the census of
                                   # the frames, the extended problem's chain and the wall regime's measurements
./target/release/taxman help
```

Cargo's commands run in `solver/frame`.  The commands of the other sections run from the top of the repository, where
`results/` is, and `taxman` in them is `solver/frame/target/release/taxman`.

The binary carries the table of optimal games built in: `results/optimal.json` as it was when the binary was compiled,
the games 1 to 1300 ([results/README.md](../../results/README.md#optimaljson) says where they came from).  Every run
checks its scores against that table, wherever the binary runs; only `verify-optimal FILE` and `extend-optimal --table
FILE` name another.  A changed table takes a rebuild, which cargo does by itself when the file has changed.

*Details.*  The study tools, the examples `tune`, `extended`, `structure`, `finder` and `wall`, are behind the feature `studies`,
which is off by default.  So a build, `cargo test` and `cargo clippy --all-targets` do not compile them, and a command
that names one (`cargo build --release --example wall`) needs `--features studies` too.  The build is long for a program
of this size because the search is compiled once for every set width
([solver/docs/design/architecture.md](../docs/design/architecture.md), the paragraph "Width dispatch").  `taxman
bench` finds its two
files in the repository: above the working directory and, if they are not there, above the binary.

*Build options.*  The release profile uses fat LTO and `panic = "abort"`.  The `test` profile is optimised
(`opt-level = 2`) because the tests replay real games.  A build is generic for its target architecture, so its binary
runs on any machine of that architecture.  Tuning for the build machine's CPU is opt-in, for a machine that will run
only that binary: `RUSTFLAGS="-C target-cpu=native" cargo build --release`, as the cloud scripts do
([RUNBOOK](../docs/RUNBOOK.md)).  It gains nothing on the Apple M1, where it was measured, and is unmeasured elsewhere;
the generic x86-64 target has no hardware popcount, so measure there.  `solver/tools/pgo.sh` makes a profile-guided
build in `target/pgo/release`, with 0 to 2% fewer cycles on the M1 and the same answers and trees.  Its header says how,
and [solver/docs/design/performance.md](../docs/design/performance.md) what it gains.  The probe, `examples/perf.rs`,
measures the hot paths.

### A first run: the games 1 to 22

The games 1 to 22 show every way the solver finds and proves a game but one.  From the top of the repository:

```
$ taxman solve 1-22 --threads 1 --progress 0 --out first.jsonl
n=1     score=0         trivial nodes=0               0.000 s  optimal ok  proof=trivial
n=2     score=2         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=3     score=3         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=4     score=7         trivial nodes=0               0.000 s  optimal ok  proof=trivial
...
n=17    score=93        prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=18    score=111       start   nodes=0               0.000 s  optimal ok  proof=cap
n=19    score=113       prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=20    score=124       search  nodes=1               0.007 s  optimal ok  proof=exhausted
n=21    score=144       search  nodes=1               0.015 s  optimal ok  proof=exhausted
n=22    score=166       repeat  nodes=0               0.000 s  optimal ok  proof=same_search of 21
```

Each line is one game: n, its score, how the solver found it, the search's steps, the seconds, a check against the
built-in table and the proof.  [The reference, section 3](../docs/REFERENCE.md#3-reading-the-output) reads a line field
by field.  The methods:

* `trivial` (games 1, 4, 6 and others): the game has no candidate to promote, so the game with no promotions is the
  only one.
* `prime` (games 2, 3, 5 and every other prime): the [prime formula](../../docs/GLOSSARY.md#the-game) gives the game of
  n from the game of n − 1.
* `start` with the proof `cap` (game 18): the search's starting game reached the
  [ceiling](../../docs/GLOSSARY.md#the-game) of game 18, 18 + 93 = 111, before it took a step.
* `search` with the proof `exhausted` (games 20 and 21): the search explored its whole tree.
* `repeat` (game 22): game 22 poses the same search problem as game 21, which settled it, so the solver takes 21's
  promotions without a search.

The sixth method, `known`, takes a game that the repository has already proven (section 3, "Known games").  "optimal
ok" says that the score equals the table's: a check, not a proof.

`--show` prints a game's promotions and moves.  Solved alone, game 21 is the example of section 1:

```
$ taxman solve 21 --show --progress 0
note: no previous game for n=21 (the game of 20 is neither in this run nor in the results file): solving it standalone, with no seed and no ceiling, which costs more; to chain, give --out with the earlier games, or --use-prev to take the known game of 20
n=21    score=144       search  nodes=1               0.001 s  optimal ok  proof=exhausted
  promotions: [9]
  moves: [19, 9, 15, 21, 14, 18, 12, 20, 16]
```

The note says that game 21 has no game of 20 to start from here, so the solver solves it *standalone*.  Finally, the two
verifiers check the file the first command wrote.  Each replays every game and derives every proof again ([the
reference, section 4](../docs/REFERENCE.md#4-why-the-answers-can-be-trusted)):

```
$ taxman verify-results first.jsonl
first.jsonl: 22 completed games, n = 1..=22 (0 missing), 0 incomplete lines
  proof: trivial 10, prime_formula 8, cap 1, exhausted 2, same_search 1
  assumptions: none in 22 of 22 games
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 9, prime formula 7
0 games are in more than one file, with equal scores in all
checked in 185.3 ms
$ python3 solver/tools/check_results.py first.jsonl
...
all checks passed
```

### Tests

From `solver/frame`:

```
python3 -I ../tools/quick_check.py   # the quick check: formatting, lints and the tests, the long ones skipped, within time limits
cargo test                           # the same tests, a binary at a time
cargo test --release                 # the full suite
```

`cargo test` runs as many tests at once as the machine has CPUs.  None of these builds the study tools;
`--features studies` adds them and their tests.  [solver/docs/design/tests.md](../docs/design/tests.md) says what the
suite checks, and [testing.md](../docs/design/testing.md) what each test takes and when each runs.  `tests/tripwires.rs`
asserts exact node counts, so a change that alters the search tree on purpose fails it; the module comment of
`tests/tripwires.rs` says how to update the counts.

The ignored tests are for large machines and long runs:

* `cargo test --release --test engine -- --ignored many_threads`: 64 and 192 worker threads;
* `TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored stress`: random engine configurations through
  resumes;
* `TXENGINE_CONSERVATION=500 cargo test --release --test robustness conservation`;
* the benchmarks under `bench` in `src/` and `tests/engine.rs`.

## 3. Commands

`taxman help` lists every command and flag.  This section shows `solve` with sample output, and
[REFERENCE.md](../docs/REFERENCE.md) the other commands.  A [tactic](../../docs/GLOSSARY.md#the-search) is set with
`--tactic NAME=VALUE`, which is repeatable; `taxman help` and
[solver/docs/design/tactics.md](../docs/design/tactics.md) give the names, values and defaults, and switching one never
changes the answer.  [The reference](../docs/REFERENCE.md#1-the-flags-of-solve) has every flag of `solve`.

### Solve one game

```
$ taxman solve 480 --use-prev --force-search --show
n=480   score=73120     search  nodes=37              0.005 s  optimal ok  proof=exhausted
  promotions: [68, 138, 147, 153, 168, 175, 180, 182, 190, 195, 196, 198, 200, 207, 208, 210, 220, 222, 224, ...]
  moves: [479, 361, 437, 299, 403, 341, 473, 301, 469, 335, 445, 267, 471, 314, 478, 466, 458, 454, 446, ...]
$ taxman solve 480 --use-prev
n=480   score=73120     known   nodes=0               0.001 s  optimal ok  proof=known from the committed chain (strict.jsonl)
```

`--use-prev` draws on the *known games*, the games the repository has already proven (below, "Known games").  The known
game of n − 1 seeds the search and gives it a ceiling.  A game that a known source has proven is recorded as it is,
with no search: that is the second command.  The first, with `--force-search`, searches 480 from the known game of
479.  Without `--use-prev` a single game has no game before it, and is solved standalone (below, "Past n = 1300").

The node count of a multi-threaded search varies a little from run to run.  "optimal ok" means that the score equals
the table's, not that it is proven ([the reference, section 4](../docs/REFERENCE.md#4-why-the-answers-can-be-trusted)).

### Solve a range

```
$ taxman solve 1-12 --show
n=1     score=0         trivial nodes=0               0.000 s  optimal ok  proof=trivial
  promotions: []
  moves: []
n=2     score=2         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
  ...
n=12    score=50        trivial nodes=0               0.000 s  optimal ok  proof=trivial
```

Each game of a range is built from the one before it.  Moving from one game to the next often leaves the search
untouched: the game is a [repeat](../../docs/GLOSSARY.md#results-and-trust), concluded at once with the promotions of
the earlier game that posed the same search problem ([the reference, "Proof kinds"](../docs/REFERENCE.md#proof-kinds)).
About half of the committed chain's games that need a search are repeats
([results/README.md](../../results/README.md#strictjsonl) has the counts).

With `--out FILE`, each result is appended to the file as soon as it is done: one JSON line per game, flushed and
synced.  A rerun skips every game the file already completed, whatever the tactics of the rerun, since the file's lines
keep the tactics they were made with and what they assume:

```
$ taxman solve 1-300 --out mine.jsonl --checkpoint-dir ckpt
$ taxman solve 1-300 --out mine.jsonl
skipped 300 games already done in mine.jsonl
```

### Past n = 1300

`optimal.json` stops at 1300, so a game above 1300 needs the game before it from the run or from the results file.
Either extend a copy of a file that has it, or give `--use-prev`, which takes the known game of n − 1.  That is the
game of `results/strict.jsonl`, or of the bounds store, where the store has proven the game.  A copy of
`results/strict.jsonl` extended is such a file.  The copy matters: `--out` appends to the file, and `results/` must not
be overwritten.

The games 1301 and 1302 are known, from `results/strict.jsonl`.  So to search them from the known game of 1300, as a
range past the table does, `--force-search` leaves them to their searches:

```
$ taxman solve 1301-1302 --use-prev --force-search --threads 2 \
      --out mine.jsonl --checkpoint-dir ckpt --node-limit 4000 --progress 4
n=1301 0.1s nodes 2979 (done) best 40800 queue 0 spilled 0 | eff 95.4% idle 4.4% lock 0.0% ...
n=1301  score=532272    search  nodes=2979            0.191 s  optimal unknown  proof=exhausted
n=1302 0.1s nodes 3723 (done) best 41436 queue 0 spilled 0 | eff 96.3% idle 3.6% lock 0.0% ...
n=1302  score=533559    search  nodes=3723            0.178 s  optimal unknown  proof=exhausted
```

Game 1301 is a prime, but `--force-search` also skips the prime formula.  Game 1302 started from 1301.  A node limit
would have stopped either with its checkpoint; the engine checks the limit every 1,024 steps of a thread.

A game whose predecessor is in neither the run nor the results file, and not taken with `--use-prev`, runs *standalone*,
with no seed and no ceiling: for example `taxman solve 1400`, or `taxman solve 480` without the flag.  `solve` says so
on stderr (`note: no previous game for n=...`), and the result line has `"prev": null`.  The answer is still correct,
but costs about twice as much on the hard games, and many times more on the games that a chain settles at once by
reaching the ceiling ([the cost curve](../notebook/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md)).

### Known games

`--use-prev` draws on three sources of games that the run has not made.  All three are in the repository's
`results/`, and no option names another:

| source | holds | a game of it rests on |
|---|---|---|
| `optimal.json` (built in) | the table of optimal games, n ≤ 1300 | nothing but the table's word: the line records that it assumes the game optimal |
| `results/strict.jsonl` | the committed chain, each game with its proof | what its line assumes |
| `results/bounds.jsonl` | the bounds the ledger found ([results/README.md](../../results/README.md#boundsjsonl)); a game whose best lower bound is its best upper bound is *proven* | statement U at the games its upper bound rests on (`rests_on`), one assumption each |

Of a game that several sources hold, the solver takes the committed chain's, then the store's, then the table's.  What
it does with them:

* **A game that a source has proven is recorded with no search.**  Its line has the method `known`, the proof `known`,
  no `setup`, and a `known` field that cites the source ([the reference, "Citing a known
  source"](../docs/REFERENCE.md#citing-a-known-source)).  The referee replays it like every game, and it is checked
  against the game before it (`score(n − 1) <= score(n) <= n + score(n − 1)`) and against the table.  Game n + 1 has the
  game of n for its ceiling and seed as usual, and a proof that rests on it, by the ceiling or the prime formula,
  assumes what it does.
* **The game before the range** is the known game of A − 1, if `--out` lacks it.  The `prev` of the first line says
  where it came from: `optimal`, `strict` or `bounds`.
* **A game that the store has not proven** still helps its search, whether the store has a lower bound, an upper bound
  or both with a gap between them.  The game of its best lower bound is a seed beside the game of n − 1, and the one
  that keeps more is used.  Its best upper bound is a ceiling next to the one that the game of n − 1 gives, and a
  search that reaches the lower of the two stops.  A search that stopped at the store's bound is proven by it: proof
  `known`, with `known` citing the bound, assuming U where the bound rests.
* `--force-search` leaves the games to their searches, and the store does not help them, as it leaves the prime
  formula and the repeats.  `regress --use-prev` takes the game before its range from the table only.

A chain from n = 1 with `--use-prev` takes its games from the committed chain as far as that goes, so it proves nothing
again there.  That is for a run that goes past the committed chain
([solver/docs/runbook/wall.md](../docs/runbook/wall.md)), not for one that checks the build.

### A chain from n = 1

```
$ taxman solve 1-2175 --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 60
```

Started at n = 1, a chain takes no game from `optimal.json`, which it only compares with, up to 1300, where the table
ends; a score that differs is a hard error.  So no proof in the file assumes anything beyond the frame model.  A run
that takes a game from `optimal.json` (`--use-prev`) records, in every result whose proof rests on it, that it assumes
that game optimal ("Known games", above).

`results/strict.jsonl` is such a chain.  This command, up to the paths, the thread count and the progress reports,
made its games 1 to 2175, and a run of it reproduces their scores.  [results/runs.md](../../results/runs.md) says how
the rest were made.

### Games in flight

```
$ taxman solve 1-2175 --threads 64 --window 4 --out strict.jsonl --checkpoint-dir ckpt
```

With `--window K`, a range keeps up to K consecutive games in flight, so that a machine of many threads stays busy
where one game's search cannot use them all.  Game n + 1 is set up while n searches, and its search starts from n's
best game so far, without the ceiling.  It gets the ceiling, with n's final game as a seed, the moment n is concluded.
While the searches run, the threads move between them, to the oldest game that keeps them busy.

Games are still concluded, checked and recorded in order.  Every line has the score, proof, repeat and assumptions that
a range solved one game after another gives it.  The moves may be another optimal set, and the nodes and times differ
([solver/docs/design/engine.md, "The window: games in flight"](../docs/design/engine.md#the-window-games-in-flight) and
[solver/docs/design/trust.md, "What a window changes in the
results"](../docs/design/trust.md#what-a-window-changes-in-the-results)).

The default window is 4, the best measured on 64 threads, and what `solver/tools/cloud/chain.sh` runs with.  With one
thread the window is always 1, one game after another.

With the environment variable `TAXMAN_WINDOW_TRACE` set, the run prints the window's events on stderr: games set up,
searches started, threads moved, deferrals, hints and conclusions.

*Details.*  On 4 threads of the laptop, a window of 16 makes 1..1300 about 16% faster, and the games past 2225, whose
searches already keep 4 threads busy, no slower ([round
B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)).  On 64 threads, a window of 16 makes the games
past 2225 20 to 32% slower than one game after another ([scaling to 192
threads](../notebook/scaling-to-192-threads-2026-10/README.md)).

### Checkpoints, resuming, signals

With `--checkpoint-dir DIR`, a game's search writes `DIR/nNNNN.ckpt` every `--checkpoint-interval` seconds (default
60), and on any stop, atomically and synced.  The same command resumes it:

```
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --progress 1 --out r992.jsonl --checkpoint-dir c992
n=992 0.3s nodes 4095 (stopped) best 24175 queue 12 spilled 0 | eff 94.3% ... pauses 1 (5.6%) max 14.0ms park 0.0ms | nogoods learned 356 ...
n=992   score=310675    search  nodes=4095            0.300 s  incomplete
stopped: the search for n=992 hit a limit before it finished; run the same command to resume it
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --out r992.jsonl --checkpoint-dir c992   # resumes, stops again
$ taxman solve 992 --use-prev --force-search --threads 1 --out r992.jsonl --checkpoint-dir c992   # resumes and finishes
n=992   score=310690    search  nodes=14789           0.454 s  optimal ok  proof=exhausted
```

The last line is the third invocation of the game: its `nodes` cover all three, its seconds only the last.  The results
file then holds two incomplete lines and one completed line for game 992, and `taxman verify-results r992.jsonl` counts
them.

* **Signals.**  SIGINT or SIGTERM stops the search as a limit does: a final checkpoint, an incomplete result line, and
  the exit status 130 or 143 (128 + the signal).  Nothing is lost.  A second signal kills the process.  SIGKILL, a crash
  or a power loss loses at most one checkpoint interval.
* **One process per checkpoint directory, and per results file.**  The directory holds a `taxman.lock`, taken with
  `flock`, and a second process is refused, naming the first.  `--out` is locked the same way while the run appends to
  it.  Keep the directory on a local disk that survives a restart, not on tmpfs.
* **A checkpoint belongs to its game and its search tactics.**  A checkpoint made for another game, by a build with a
  different frame builder, or with other search tactics is refused, saying what differs.  Resume with the same
  `--tactic` flags, or delete the file.
* **Limits.**  `--time-limit S` and `--node-limit N` bound one game's search in one invocation, not the whole range.  A
  game that a limit stops gets an incomplete result line (`completed` false, with `best_so_far`), and the range stops
  there with the exit status 0.  With `--keep-going` it goes on (next subsection).
* **The wall.**  `--game-hours H` ends the range at the first game whose own search passes H hours.  The hours are
  summed over every invocation, and counted only while the game is the oldest of the window with its search running,
  not while it waits behind another.  Every search in flight stops with a final checkpoint, and nothing is recorded for
  that game, not even an incomplete line.  A search that completes before the stop reaches it is recorded, and the
  range goes on.  `solve` prints `wall at N after H hours` and exits with the status 4.  The game's clock is kept
  beside its checkpoint (`nNNNN.clock`), so the same command with a larger H goes on from both
  ([design/engine.md](../docs/design/engine.md#the-window-games-in-flight)).
* **A finder beside a long search.**  `--finder-after NODES` starts a thread of its own, beside the workers, once a game's
  search has taken NODES steps (over all its invocations), which looks for a better game than the incumbent and offers it
  to the search; `--finder-seconds S` is its budget (default 60).  It changes no answer, and it is not part of a
  checkpoint: a checkpoint resumes with any finder or none.  The line records what it did (`engine.finder`, [the
  reference](../docs/REFERENCE.md#a-results-line---out-schema-5)); [the design](../docs/design/search.md#the-finder) says
  how it works.
* **A result is recorded before its checkpoint is deleted**, so a crash never loses a finished game.  At worst the
  checkpoint of a finished game is left over, and the next run deletes it.
* **Memory.**  A searcher's state is bounded.  The shared queue of work units spills to `DIR/spill` beyond `--mem-cap`
  units (default 1048576).  The runs of the committed chain never came near it
  ([results/runs.md](../../results/runs.md) gives their largest queues).

### Keeping going past limits, and the sweep

```
$ taxman solve 3441-3600 --threads 1 --time-limit 180 --keep-going --out part.jsonl
```

With `--keep-going`, a game whose search a limit stops does not end the range.  Its best game so far is recorded as an
unfinished line (`completed` false, with `best_so_far` and the moves).  The next game goes on from that game as a seed
and nothing more, since it is not known to be optimal: it gets no ceiling n + score(n − 1) from it, no prime formula,
and no proof that rests on it.  Its line says so: its `prev` has no proof ([the reference, "What a game had from n −
1"](../docs/REFERENCE.md#what-a-game-had-from-n---1)).  What does not rest on the game before is as ever: an exhausted
search is a proof, and so is a repeat of a completed game of the file.  The games are solved one at a time (`--window`
is 1).

* **A rerun** on the same `--out` passes over the games that the file holds only unfinished lines of, and solves the
  others.  It seeds the game after the last of them with its best game, the best of its lines.
* **`--retry`** searches the unfinished games again instead, each from its checkpoint if `--checkpoint-dir` kept one,
  as a second pass with a longer limit does.
* **`--retry-improved`** searches again only the unfinished games whose game before has more to give than it had when
  they were tried: it is proven now, or has a better best game than their seed, or they had none.  It passes over the
  rest.  The games it searches again go in increasing n, so that one it has just proven helps the next.
* **A signal** still ends the range, but with no line for the game it stopped, so that a rerun resumes it.
* **`--pass N`** records N in `options.pass` of every line the run writes, and changes nothing else.

**The sweep** (`solver/tools/cloud/sweep.sh FIRST LAST`, [solver/docs/runbook/sweep.md](../docs/runbook/sweep.md))
goes over a long stretch of games for as many proven games as the time allows, holes allowed.  It splits the stretch
into blocks of consecutive games of about equal expected work.  Each block is one `taxman solve --keep-going` on one
thread, with a limit on each game's search (`GAME_SECONDS`, 180).  Then passes of `--retry-improved` go over the
merged file, so that each game sees the game before it whichever block made it, until a pass changes nothing.

Its file, `explore.jsonl`, holds every line of every block and pass.  It is **not a chain**, and never goes into
`results/strict.jsonl`: some of its games have only an unfinished search, or none, and each proof in it rests only on
the games it holds.  Both verifiers accept it ([the reference, "The verifiers"](../docs/REFERENCE.md#the-verifiers)).

**The ledger** works on a whole stretch of games at once instead of in order.  For every game it keeps a lower bound
with a game that reaches it, and an upper bound, each with how it was derived, and a game is proven when its bounds
meet.  It tightens them where a proof looks cheapest (its *forecast*), in three ways:

* limited one-thread searches, P at once, each going on from its checkpoint, with its upper bound read from the
  checkpoint (the *dual bound*);
* searches under floors that step an open game's upper bound down, each costing about tenfold more than the one before
  and giving the game a line that forecasts its proof;
* the relations between games, both ways: the ceiling, the prime formula, repeats and monotonicity.

A finder of good games, a large neighbourhood search, adds lower bounds.  It is not sound, so its games count only once
the referee has replayed them.  The ledger never reads the known optima unless it is given proven games (`--proven
FILE`), so a run on a stretch that the chain has proven is a fair test, which `report --known` checks.

```
$ cargo run --release --example ledger -- run A-B --dir DIR --cpu-hours H --workers P
$ cargo run --release --example ledger -- report DIR --known FILE
```

The module comment of `examples/ledger/main.rs` has the flags and the files.  What the ledger finds is kept in the
bounds store, `results/bounds.jsonl` (`ledger export`; `run --bounds` starts from it), which has two verifiers of its
own ([results/README.md](../../results/README.md#boundsjsonl)) and which `taxman solve --use-prev` draws on too ("Known
games", above).  The ledger's files are its own, not results lines ([solver/docs/design/trust.md, "The
ledger"](../docs/design/trust.md#the-ledger); the experiment: [the
ledger](../notebook/search/the-ledger-bounds-from-both-sides-2026-10/README.md)).
